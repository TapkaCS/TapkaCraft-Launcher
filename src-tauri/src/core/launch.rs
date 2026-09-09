//! `LaunchEngine` (Phase 3).
//!
//! Turns a resolved instance + authenticated account into a running
//! Minecraft process: verifies the selected Java runtime satisfies the
//! version's requirement, builds the classpath, JVM arguments and game
//! arguments as an argument list (never hand-built string concatenation),
//! and spawns `java`/`javaw`. Streams stdout/stderr back to the caller as
//! a live log and reports the exit code so `CrashDoctor` (Phase 8) has
//! something to analyze on a non-zero exit.
//!
//! `AuthSession` is only ever *consumed* here - Phase 4's `AccountService`
//! is the only thing allowed to construct one, from a real, completed
//! Microsoft/Xbox/Minecraft-Services sign-in. Nothing in this module (or
//! anywhere else in Phase 3) builds a fake/offline session as a fallback;
//! the test fixtures below build one explicitly with obviously-fake data,
//! the same way they build a throwaway `.jar`, never as a runtime default.

use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::process::Stdio;

use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;

use super::instances::model::JavaSettings;
use super::java::runtime::DetectedRuntime;
use super::versions::install::InstalledVersion;
use super::versions::rules::RuleContext;
use super::versions::VersionError;

/// A completed Microsoft/Xbox/Minecraft-Services sign-in. `client_id` is
/// the launcher's own Azure app registration id (the `${clientid}`
/// placeholder some versions pass through to Microsoft's session/telemetry
/// APIs) - not a per-player value.
#[derive(Debug, Clone)]
pub struct AuthSession {
    pub player_name: String,
    pub uuid: String,
    pub access_token: String,
    pub xuid: String,
    pub client_id: String,
}

#[derive(Debug)]
pub enum LaunchError {
    JavaTooOld {
        required_major: u32,
        found_major: u32,
    },
    Version(VersionError),
    Io(std::io::Error),
}

impl fmt::Display for LaunchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JavaTooOld {
                required_major,
                found_major,
            } => write!(
                f,
                "This version needs Java {required_major} or newer, but the selected runtime is Java {found_major}."
            ),
            Self::Version(err) => write!(f, "{err}"),
            Self::Io(err) => write!(f, "Couldn't start Minecraft: {err}"),
        }
    }
}

impl std::error::Error for LaunchError {}

impl From<std::io::Error> for LaunchError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl Serialize for LaunchError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum LaunchEvent {
    Started { pid: u32 },
    Output { stream: OutputStream, line: String },
    Exited { code: Option<i32> },
}

/// Where `LaunchEvent`s go - mirrors `core::downloads::ProgressSink` so
/// callers that don't care (most tests) can use `LaunchEventSink::none()`
/// instead of threading `Option` handling through every call site.
#[derive(Clone)]
pub struct LaunchEventSink(Option<tokio::sync::mpsc::UnboundedSender<LaunchEvent>>);

impl LaunchEventSink {
    pub fn new(sender: tokio::sync::mpsc::UnboundedSender<LaunchEvent>) -> Self {
        Self(Some(sender))
    }

    pub fn none() -> Self {
        Self(None)
    }

    fn send(&self, event: LaunchEvent) {
        if let Some(sender) = &self.0 {
            let _ = sender.send(event);
        }
    }
}

/// Everything `launch` needs beyond the version's own metadata. Grouped
/// into a struct (rather than nine positional parameters) purely for
/// readability at the call site.
pub struct LaunchParams<'a> {
    /// The instance's own directory - becomes both the process's working
    /// directory and the `${game_directory}` argument, so saves/mods/config
    /// land where the instance (not some shared/global location) expects.
    pub instance_dir: &'a Path,
    pub java_settings: &'a JavaSettings,
    pub installed: &'a InstalledVersion,
    pub libraries_dir: &'a Path,
    pub assets_dir: &'a Path,
    pub runtime: &'a DetectedRuntime,
    pub auth: &'a AuthSession,
    pub launcher_name: &'a str,
    pub launcher_version: &'a str,
}

/// Spawns the Minecraft client process and waits for it to exit, streaming
/// every stdout/stderr line to `events` as it's produced. `Ok` carries the
/// process's exit code (`-1` if the OS reports none, e.g. killed by a
/// signal on Unix).
pub async fn launch(params: LaunchParams<'_>, events: LaunchEventSink) -> Result<i32, LaunchError> {
    if let Some(requirement) = &params.installed.version_info.java_version {
        if params.runtime.major_version < requirement.major_version {
            return Err(LaunchError::JavaTooOld {
                required_major: requirement.major_version,
                found_major: params.runtime.major_version,
            });
        }
    }

    // Custom resolution isn't wired up to instance settings yet - Minecraft
    // falls back to its own default window size when `--width`/`--height`
    // are absent from the game arguments.
    let ctx = RuleContext::current(false);

    let classpath = build_classpath(params.installed, params.libraries_dir, &ctx);
    let substitutions = build_substitutions(&params, &classpath);

    let mut args: Vec<String> = vec![
        format!("-Xms{}M", params.java_settings.memory_min_mb),
        format!("-Xmx{}M", params.java_settings.memory_max_mb),
    ];
    args.extend(
        params
            .installed
            .version_info
            .jvm_arguments(&ctx)
            .iter()
            .map(|arg| substitute(arg, &substitutions)),
    );
    args.push(params.installed.version_info.main_class.clone());
    args.extend(
        params
            .installed
            .version_info
            .game_arguments(&ctx)
            .map_err(LaunchError::Version)?
            .iter()
            .map(|arg| substitute(arg, &substitutions)),
    );

    let mut child = Command::new(params.runtime.launch_executable())
        .args(&args)
        .current_dir(params.instance_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    events.send(LaunchEvent::Started {
        pid: child.id().unwrap_or(0),
    });

    let stdout = child.stdout.take().expect("stdout was requested as piped");
    let stderr = child.stderr.take().expect("stderr was requested as piped");
    let stdout_task = tokio::spawn(stream_lines(stdout, OutputStream::Stdout, events.clone()));
    let stderr_task = tokio::spawn(stream_lines(stderr, OutputStream::Stderr, events.clone()));

    let status = child.wait().await?;
    // The process has exited, so both pipes are at EOF - these complete
    // almost immediately and only exist to make sure every already-flushed
    // line was forwarded before we report `Exited`.
    let _ = stdout_task.await;
    let _ = stderr_task.await;

    let code = status.code();
    events.send(LaunchEvent::Exited { code });
    Ok(code.unwrap_or(-1))
}

async fn stream_lines<R: AsyncRead + Unpin>(
    pipe: R,
    stream: OutputStream,
    events: LaunchEventSink,
) {
    let mut lines = BufReader::new(pipe).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        events.send(LaunchEvent::Output { stream, line });
    }
}

/// Every applicable library's jar, in declaration order, followed by the
/// client jar itself - joined with the platform's classpath separator
/// (`;` on Windows, `:` elsewhere).
fn build_classpath(
    installed: &InstalledVersion,
    libraries_dir: &Path,
    ctx: &RuleContext,
) -> String {
    let separator = if cfg!(windows) { ";" } else { ":" };
    let mut entries: Vec<String> = installed
        .version_info
        .classpath_artifacts(ctx)
        .iter()
        .map(|artifact| {
            libraries_dir
                .join(&artifact.path)
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    entries.push(installed.client_jar.to_string_lossy().into_owned());
    entries.join(separator)
}

fn build_substitutions(
    params: &LaunchParams<'_>,
    classpath: &str,
) -> HashMap<&'static str, String> {
    let info = &params.installed.version_info;
    let separator = if cfg!(windows) { ";" } else { ":" };

    let mut values = HashMap::new();
    values.insert(
        "natives_directory",
        path_string(&params.installed.natives_dir),
    );
    values.insert("launcher_name", params.launcher_name.to_string());
    values.insert("launcher_version", params.launcher_version.to_string());
    values.insert("classpath", classpath.to_string());
    values.insert("classpath_separator", separator.to_string());
    values.insert("library_directory", path_string(params.libraries_dir));
    values.insert("version_name", info.id.clone());
    values.insert(
        "version_type",
        info.version_type
            .clone()
            .unwrap_or_else(|| "release".to_string()),
    );
    values.insert("game_directory", path_string(params.instance_dir));
    values.insert("assets_root", path_string(params.assets_dir));
    values.insert("assets_index_name", info.asset_index.id.clone());
    values.insert("auth_player_name", params.auth.player_name.clone());
    values.insert("auth_uuid", params.auth.uuid.clone());
    values.insert("auth_access_token", params.auth.access_token.clone());
    // Pre-1.7 versions use `${auth_session}` instead of a separate access
    // token argument - same value, older placeholder name.
    values.insert("auth_session", params.auth.access_token.clone());
    values.insert("auth_xuid", params.auth.xuid.clone());
    values.insert("clientid", params.auth.client_id.clone());
    // Mojang/offline accounts don't exist here - TapkaCraft only ever signs
    // in through Microsoft.
    values.insert("user_type", "msa".to_string());
    // Legacy placeholder from the Mojang-account era; no modern launcher
    // fills it with anything meaningful, but some old versions still ask.
    values.insert("user_properties", "{}".to_string());
    values
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// Replaces every `${key}` this launch knows a value for. A placeholder
/// with no known value is left exactly as-is rather than silently dropped -
/// a stray literal `${...}` reaching the game is a visible bug to chase,
/// a quietly-blanked one would not be.
fn substitute(template: &str, values: &HashMap<&'static str, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in values {
        result = result.replace(&format!("${{{key}}}"), value);
    }
    result
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::process::Command as StdCommand;

    use tokio::sync::mpsc;

    use super::*;
    use crate::core::java::detect;
    use crate::core::versions::version_json::VersionInfo;

    fn fake_auth() -> AuthSession {
        AuthSession {
            player_name: "TestPlayer".into(),
            uuid: "11111111-2222-3333-4444-555555555555".into(),
            access_token: "fake-access-token".into(),
            xuid: "fake-xuid".into(),
            client_id: "fake-client-id".into(),
        }
    }

    fn version_info_requiring_java(major: u32) -> VersionInfo {
        let json = serde_json::json!({
            "id": "1.20.1-launch-test",
            "type": "release",
            "mainClass": "Main",
            "downloads": { "client": { "url": "unused://", "sha1": "unused", "size": 1 } },
            "assetIndex": { "id": "test-assets", "url": "unused://", "sha1": "unused", "size": 1 },
            "javaVersion": { "component": "java-runtime-gamma", "majorVersion": major },
            "arguments": {
                "jvm": ["-Djava.library.path=${natives_directory}", "-cp", "${classpath}"],
                "game": ["--username", "${auth_player_name}", "--uuid", "${auth_uuid}", "--gameDir", "${game_directory}"]
            }
        });
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn substitute_replaces_every_known_placeholder_and_leaves_unknown_ones_alone() {
        let mut values = HashMap::new();
        values.insert("auth_player_name", "Steve".to_string());
        values.insert("version_name", "1.20.1".to_string());

        assert_eq!(
            substitute(
                "--username ${auth_player_name} --version ${version_name}",
                &values
            ),
            "--username Steve --version 1.20.1"
        );
        assert_eq!(
            substitute("${totally_unknown}", &values),
            "${totally_unknown}"
        );
    }

    #[test]
    fn build_classpath_lists_libraries_then_the_client_jar_with_the_platform_separator() {
        let info: VersionInfo = serde_json::from_value(serde_json::json!({
            "id": "1.20.1",
            "mainClass": "Main",
            "downloads": { "client": { "url": "unused://", "sha1": "unused", "size": 1 } },
            "assetIndex": { "id": "x", "url": "unused://", "sha1": "unused", "size": 1 },
            "libraries": [
                { "name": "com.mojang:brigadier:1.0.18", "downloads": { "artifact": { "path": "com/mojang/brigadier/1.0.18/brigadier.jar", "url": "unused://", "sha1": "unused", "size": 1 } } }
            ]
        }))
        .unwrap();
        let installed = InstalledVersion {
            version_info: info,
            client_jar: PathBuf::from("/libs/1.20.1/1.20.1.jar"),
            natives_dir: PathBuf::from("/libs/1.20.1/natives"),
        };
        let ctx = RuleContext::current(false);
        let classpath = build_classpath(&installed, Path::new("/libs"), &ctx);

        let separator = if cfg!(windows) { ";" } else { ":" };
        let expected = format!(
            "/libs/com/mojang/brigadier/1.0.18/brigadier.jar{separator}/libs/1.20.1/1.20.1.jar"
        );
        assert_eq!(classpath, expected);
    }

    #[tokio::test]
    async fn rejects_a_runtime_older_than_the_version_requires() {
        let installed = InstalledVersion {
            version_info: version_info_requiring_java(17),
            client_jar: PathBuf::from("/unused/client.jar"),
            natives_dir: PathBuf::from("/unused/natives"),
        };
        let runtime = DetectedRuntime {
            java_path: PathBuf::from("java"),
            major_version: 8,
            version_string: "1.8.0_431".into(),
            is_64_bit: true,
            vendor: None,
        };
        let java_settings = JavaSettings {
            memory_min_mb: 256,
            memory_max_mb: 512,
        };
        let auth = fake_auth();
        let params = LaunchParams {
            instance_dir: Path::new("/unused"),
            java_settings: &java_settings,
            installed: &installed,
            libraries_dir: Path::new("/unused/libraries"),
            assets_dir: Path::new("/unused/assets"),
            runtime: &runtime,
            auth: &auth,
            launcher_name: "TapkaCraft Launcher",
            launcher_version: "0.1.0",
        };

        let result = launch(params, LaunchEventSink::none()).await;
        assert!(matches!(
            result,
            Err(LaunchError::JavaTooOld {
                required_major: 17,
                found_major: 8
            })
        ));
    }

    fn find_jdk_bin_dir() -> Option<PathBuf> {
        let path_var = std::env::var_os("PATH")?;
        let javac_name = if cfg!(windows) { "javac.exe" } else { "javac" };
        std::env::split_paths(&path_var).find(|dir| dir.join(javac_name).is_file())
    }

    /// Real integration check, in the same spirit as
    /// `java::detect::tests::verify_works_against_this_machine_s_actual_java_if_one_is_installed`:
    /// compiles and packages an actual tiny Java program, then runs it
    /// through the real `launch` pipeline end to end - real process spawn,
    /// real argument substitution (the fake player name has to survive
    /// into the child's actual argv), real stdout/stderr streaming over a
    /// real OS pipe, real exit code. Skips (doesn't fail) if this
    /// environment has no JDK on PATH, the same "best-effort" posture the
    /// existing JavaManager test uses.
    #[tokio::test]
    async fn launches_a_real_java_process_streams_its_output_and_reports_a_clean_exit() {
        let Some(java_dir) = find_jdk_bin_dir() else {
            eprintln!("no JDK with javac found on PATH - skipping real-process LaunchEngine test");
            return;
        };
        let exe = |name: &str| {
            java_dir.join(if cfg!(windows) {
                format!("{name}.exe")
            } else {
                name.to_string()
            })
        };
        let javac = exe("javac");
        let jar_tool = exe("jar");

        let Some(runtime) = detect::verify(&exe("java")) else {
            eprintln!("found javac but couldn't verify `java -version` - skipping");
            return;
        };

        let work_dir = tempfile::tempdir().unwrap();
        let src_path = work_dir.path().join("Main.java");
        std::fs::write(
            &src_path,
            r#"
            public class Main {
                public static void main(String[] args) {
                    System.out.println("ARGS:" + String.join("|", args));
                    System.err.println("STDERR_LINE");
                    System.out.println("MAIN_RAN_OK");
                }
            }
            "#,
        )
        .unwrap();

        let classes_dir = work_dir.path().join("classes");
        std::fs::create_dir_all(&classes_dir).unwrap();
        let javac_status = StdCommand::new(&javac)
            .arg("-d")
            .arg(&classes_dir)
            .arg(&src_path)
            .status()
            .unwrap();
        assert!(
            javac_status.success(),
            "javac failed to compile the test fixture"
        );

        let jar_path = work_dir.path().join("test-client.jar");
        let jar_status = StdCommand::new(&jar_tool)
            .arg("cf")
            .arg(&jar_path)
            .arg("-C")
            .arg(&classes_dir)
            .arg("Main.class")
            .status()
            .unwrap();
        assert!(
            jar_status.success(),
            "jar failed to package the test fixture"
        );

        let instance_dir = work_dir.path().join("instance");
        std::fs::create_dir_all(&instance_dir).unwrap();
        let natives_dir = work_dir.path().join("natives");
        std::fs::create_dir_all(&natives_dir).unwrap();
        let libraries_dir = work_dir.path().join("libraries");
        let assets_dir = work_dir.path().join("assets");

        let installed = InstalledVersion {
            version_info: version_info_requiring_java(runtime.major_version),
            client_jar: jar_path,
            natives_dir,
        };
        let java_settings = JavaSettings {
            memory_min_mb: 32,
            memory_max_mb: 64,
        };
        let auth = fake_auth();
        let (tx, mut rx) = mpsc::unbounded_channel();

        let params = LaunchParams {
            instance_dir: &instance_dir,
            java_settings: &java_settings,
            installed: &installed,
            libraries_dir: &libraries_dir,
            assets_dir: &assets_dir,
            runtime: &runtime,
            auth: &auth,
            launcher_name: "TapkaCraft Launcher",
            launcher_version: "0.1.0-test",
        };

        let exit_code = launch(params, LaunchEventSink::new(tx)).await.unwrap();
        assert_eq!(exit_code, 0);

        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }

        assert!(matches!(events.first(), Some(LaunchEvent::Started { pid }) if *pid > 0));

        let stdout_lines: Vec<&str> = events
            .iter()
            .filter_map(|e| match e {
                LaunchEvent::Output {
                    stream: OutputStream::Stdout,
                    line,
                } => Some(line.as_str()),
                _ => None,
            })
            .collect();
        // Proves game arguments were built *and* substituted for real (the
        // fake player name shows up in the child's actual argv, not just
        // in our own source) and that stdout was captured line-by-line
        // through a real OS pipe.
        assert!(
            stdout_lines
                .iter()
                .any(|line| line.contains("ARGS:") && line.contains("TestPlayer")),
            "stdout lines were: {stdout_lines:?}"
        );
        assert!(stdout_lines.contains(&"MAIN_RAN_OK"));

        let stderr_lines: Vec<&str> = events
            .iter()
            .filter_map(|e| match e {
                LaunchEvent::Output {
                    stream: OutputStream::Stderr,
                    line,
                } => Some(line.as_str()),
                _ => None,
            })
            .collect();
        assert!(stderr_lines.contains(&"STDERR_LINE"));

        assert!(matches!(
            events.last(),
            Some(LaunchEvent::Exited { code: Some(0) })
        ));
    }
}
