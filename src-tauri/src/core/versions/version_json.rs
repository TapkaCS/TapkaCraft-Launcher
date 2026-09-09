//! The per-version metadata JSON (what a `VersionManifestEntry.url` points
//! to): libraries, downloads, launch arguments, Java requirement. Mirrors
//! the real Mojang "client.json" shape, including the two argument formats
//! it has used over Minecraft's history.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::rules::{rules_allow, Rule, RuleContext};
use super::VersionError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub id: String,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    pub downloads: VersionDownloads,
    #[serde(rename = "assetIndex")]
    pub asset_index: AssetIndexRef,
    /// "release" / "snapshot" / "old_beta" / "old_alpha" - shown in-game
    /// (bottom-right of the main menu) via the `${version_type}` launch
    /// placeholder. Optional only so hand-built test fixtures that predate
    /// this field don't need updating; every real Mojang version JSON has it.
    #[serde(rename = "type", default)]
    pub version_type: Option<String>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(rename = "javaVersion", default)]
    pub java_version: Option<JavaVersionRequirement>,
    #[serde(default)]
    pub logging: Option<LoggingInfo>,

    /// 1.13+: structured, rule-aware argument lists.
    #[serde(default)]
    pub arguments: Option<Arguments>,
    /// Pre-1.13: one shell-style string, space-separated, no rules.
    #[serde(rename = "minecraftArguments", default)]
    pub minecraft_arguments: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDownloads {
    pub client: DownloadRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadRef {
    pub url: String,
    pub sha1: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndexRef {
    pub id: String,
    pub url: String,
    pub sha1: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaVersionRequirement {
    pub component: String,
    #[serde(rename = "majorVersion")]
    pub major_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingInfo {
    pub client: LoggingClient,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingClient {
    pub argument: String,
    pub file: DownloadRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<ArgumentEntry>,
    #[serde(default)]
    pub jvm: Vec<ArgumentEntry>,
}

/// Either a bare string argument, or one gated behind `rules` whose value
/// may itself be one string or several (e.g. `["--width", "${w}"]`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgumentEntry {
    Plain(String),
    Conditional {
        rules: Vec<Rule>,
        value: ArgumentValue,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgumentValue {
    Single(String),
    Multiple(Vec<String>),
}

impl ArgumentEntry {
    /// The still-templated strings this entry contributes on the current
    /// platform (empty if a conditional entry's rules don't pass).
    fn resolve(&self, ctx: &RuleContext) -> Vec<String> {
        match self {
            ArgumentEntry::Plain(s) => vec![s.clone()],
            ArgumentEntry::Conditional { rules, value } => {
                if !rules_allow(rules, ctx) {
                    return Vec::new();
                }
                match value {
                    ArgumentValue::Single(s) => vec![s.clone()],
                    ArgumentValue::Multiple(items) => items.clone(),
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    /// Pre-modern format: maps an OS name to the `classifiers` key holding
    /// that OS's natives jar. The `${arch}` placeholder (seen on very old
    /// versions) is substituted with "32"/"64".
    #[serde(default)]
    pub natives: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub extract: Option<LibraryExtractRules>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDownloads {
    #[serde(default)]
    pub artifact: Option<LibraryArtifact>,
    #[serde(default)]
    pub classifiers: Option<std::collections::HashMap<String, LibraryArtifact>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryArtifact {
    pub path: String,
    pub url: String,
    pub sha1: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryExtractRules {
    #[serde(default)]
    pub exclude: Vec<String>,
}

impl VersionInfo {
    /// Libraries whose `rules` (if any) permit the current platform.
    pub fn applicable_libraries<'a>(&'a self, ctx: &RuleContext) -> Vec<&'a Library> {
        self.libraries
            .iter()
            .filter(|lib| rules_allow(&lib.rules, ctx))
            .collect()
    }

    /// The classpath-contributing artifact for each applicable library
    /// (its main jar - never a natives-only classifier jar).
    pub fn classpath_artifacts<'a>(&'a self, ctx: &RuleContext) -> Vec<&'a LibraryArtifact> {
        self.applicable_libraries(ctx)
            .into_iter()
            .filter_map(|lib| lib.downloads.as_ref()?.artifact.as_ref())
            .collect()
    }

    /// The natives-jar artifact for each applicable library that ships one
    /// for the current OS, alongside that library's `extract` exclusions.
    pub fn native_artifacts<'a>(
        &'a self,
        ctx: &RuleContext,
    ) -> Vec<(&'a LibraryArtifact, &'a [String])> {
        let mut result = Vec::new();
        for lib in self.applicable_libraries(ctx) {
            let Some(downloads) = &lib.downloads else {
                continue;
            };
            let Some(classifiers) = &downloads.classifiers else {
                continue;
            };
            let Some(natives_map) = &lib.natives else {
                continue;
            };
            let Some(key_template) = natives_map.get(&ctx.os_name) else {
                continue;
            };
            let arch_bits = if ctx.os_arch == "x86" { "32" } else { "64" };
            let key = key_template.replace("${arch}", arch_bits);
            if let Some(artifact) = classifiers.get(&key) {
                let exclude: &[String] = lib
                    .extract
                    .as_ref()
                    .map(|e| e.exclude.as_slice())
                    .unwrap_or(&[]);
                result.push((artifact, exclude));
            }
        }
        result
    }

    /// Still-templated JVM arguments (e.g. `-Djava.library.path=${natives_directory}`),
    /// in the order Mojang declares them. Falls back to a sane default set
    /// for the legacy (pre-1.13) format, which never specified JVM args at all.
    pub fn jvm_arguments(&self, ctx: &RuleContext) -> Vec<String> {
        match &self.arguments {
            Some(args) => args.jvm.iter().flat_map(|a| a.resolve(ctx)).collect(),
            None => vec![
                "-Djava.library.path=${natives_directory}".to_string(),
                "-cp".to_string(),
                "${classpath}".to_string(),
            ],
        }
    }

    /// Still-templated game arguments. Legacy versions store these as one
    /// space-separated string with no rule support at all.
    pub fn game_arguments(&self, ctx: &RuleContext) -> Result<Vec<String>, VersionError> {
        if let Some(args) = &self.arguments {
            return Ok(args.game.iter().flat_map(|a| a.resolve(ctx)).collect());
        }
        if let Some(legacy) = &self.minecraft_arguments {
            return Ok(legacy.split_whitespace().map(str::to_string).collect());
        }
        Err(VersionError::Parse(format!(
            "version {} has neither `arguments` nor `minecraftArguments`",
            self.id
        )))
    }
}

const VERSION_JSON_TIMEOUT: Duration = Duration::from_secs(30);

pub async fn fetch_version_info(
    client: &reqwest::Client,
    url: &str,
) -> Result<VersionInfo, VersionError> {
    let response = client
        .get(url)
        .timeout(VERSION_JSON_TIMEOUT)
        .send()
        .await
        .map_err(|err| VersionError::Network(err.to_string()))?;

    if !response.status().is_success() {
        return Err(VersionError::Network(format!(
            "version metadata request failed with HTTP {}",
            response.status()
        )));
    }

    response
        .json::<VersionInfo>()
        .await
        .map_err(|err| VersionError::Parse(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> RuleContext {
        RuleContext {
            os_name: "windows".into(),
            os_arch: "x86_64".into(),
            has_custom_resolution: false,
        }
    }

    /// A trimmed but structurally real modern (1.19-era) version JSON:
    /// one always-applicable library, one library that only exists for
    /// linux, and JVM/game arguments including a conditional entry.
    fn modern_version_json() -> &'static str {
        r#"{
            "id": "1.20.1",
            "mainClass": "net.minecraft.client.main.Main",
            "downloads": { "client": { "url": "https://example/client.jar", "sha1": "abc", "size": 100 } },
            "assetIndex": { "id": "10", "url": "https://example/10.json", "sha1": "def", "size": 50 },
            "javaVersion": { "component": "java-runtime-gamma", "majorVersion": 17 },
            "libraries": [
                {
                    "name": "com.mojang:brigadier:1.0.18",
                    "downloads": {
                        "artifact": { "path": "com/mojang/brigadier/1.0.18/brigadier.jar", "url": "https://libraries.minecraft.net/brigadier.jar", "sha1": "111", "size": 10 }
                    }
                },
                {
                    "name": "net.java.dev.jna:jna:5.10.0",
                    "rules": [ { "action": "allow", "os": { "name": "linux" } } ],
                    "downloads": {
                        "artifact": { "path": "net/java/dev/jna/jna/5.10.0/jna.jar", "url": "https://libraries.minecraft.net/jna.jar", "sha1": "222", "size": 10 }
                    }
                },
                {
                    "name": "org.lwjgl:lwjgl:3.3.1",
                    "natives": { "windows": "natives-windows", "linux": "natives-linux", "osx": "natives-macos" },
                    "extract": { "exclude": ["META-INF/"] },
                    "downloads": {
                        "artifact": { "path": "org/lwjgl/lwjgl/3.3.1/lwjgl.jar", "url": "https://libraries.minecraft.net/lwjgl.jar", "sha1": "333", "size": 10 },
                        "classifiers": {
                            "natives-windows": { "path": "org/lwjgl/lwjgl/3.3.1/lwjgl-natives-windows.jar", "url": "https://libraries.minecraft.net/lwjgl-natives-windows.jar", "sha1": "444", "size": 5 },
                            "natives-linux": { "path": "org/lwjgl/lwjgl/3.3.1/lwjgl-natives-linux.jar", "url": "https://libraries.minecraft.net/lwjgl-natives-linux.jar", "sha1": "555", "size": 5 }
                        }
                    }
                }
            ],
            "arguments": {
                "jvm": [
                    "-Djava.library.path=${natives_directory}",
                    { "rules": [ { "action": "allow", "os": { "name": "osx" } } ], "value": ["-XstartOnFirstThread"] },
                    "-cp",
                    "${classpath}"
                ],
                "game": [
                    "--username", "${auth_player_name}",
                    "--uuid", "${auth_uuid}",
                    { "rules": [ { "action": "allow", "features": { "has_custom_resolution": true } } ], "value": ["--width", "${resolution_width}", "--height", "${resolution_height}"] }
                ]
            }
        }"#
    }

    fn legacy_version_json() -> &'static str {
        r#"{
            "id": "1.8.9",
            "mainClass": "net.minecraft.client.main.Main",
            "downloads": { "client": { "url": "https://example/client.jar", "sha1": "abc", "size": 100 } },
            "assetIndex": { "id": "1.8", "url": "https://example/1.8.json", "sha1": "def", "size": 50 },
            "minecraftArguments": "--username ${auth_player_name} --version ${version_name} --uuid ${auth_uuid}",
            "libraries": [
                {
                    "name": "com.mojang:netty:1.8.8",
                    "downloads": {
                        "artifact": { "path": "com/mojang/netty/1.8.8/netty.jar", "url": "https://libraries.minecraft.net/netty.jar", "sha1": "666", "size": 10 }
                    }
                }
            ]
        }"#
    }

    #[test]
    fn parses_modern_version_json() {
        let info: VersionInfo = serde_json::from_str(modern_version_json()).unwrap();
        assert_eq!(info.id, "1.20.1");
        assert_eq!(info.java_version.unwrap().major_version, 17);
        assert_eq!(info.libraries.len(), 3);
    }

    #[test]
    fn applicable_libraries_excludes_libraries_gated_to_other_platforms() {
        let info: VersionInfo = serde_json::from_str(modern_version_json()).unwrap();
        let names: Vec<&str> = info
            .applicable_libraries(&ctx())
            .iter()
            .map(|l| l.name.as_str())
            .collect();
        assert!(names.contains(&"com.mojang:brigadier:1.0.18"));
        assert!(names.contains(&"org.lwjgl:lwjgl:3.3.1"));
        assert!(!names.contains(&"net.java.dev.jna:jna:5.10.0")); // linux-only
    }

    #[test]
    fn classpath_artifacts_include_the_main_jar_for_every_applicable_library() {
        let info: VersionInfo = serde_json::from_str(modern_version_json()).unwrap();
        let paths: Vec<&str> = info
            .classpath_artifacts(&ctx())
            .iter()
            .map(|a| a.path.as_str())
            .collect();
        assert_eq!(
            paths,
            vec![
                "com/mojang/brigadier/1.0.18/brigadier.jar",
                "org/lwjgl/lwjgl/3.3.1/lwjgl.jar",
            ]
        );
    }

    #[test]
    fn native_artifacts_picks_the_classifier_matching_the_current_os() {
        let info: VersionInfo = serde_json::from_str(modern_version_json()).unwrap();
        let natives = info.native_artifacts(&ctx());
        assert_eq!(natives.len(), 1);
        assert_eq!(
            natives[0].0.path,
            "org/lwjgl/lwjgl/3.3.1/lwjgl-natives-windows.jar"
        );
        assert_eq!(natives[0].1, ["META-INF/".to_string()]);
    }

    #[test]
    fn native_artifacts_is_empty_on_a_platform_with_no_matching_classifier() {
        let info: VersionInfo = serde_json::from_str(modern_version_json()).unwrap();
        let macos_ctx = RuleContext {
            os_name: "osx".into(),
            os_arch: "x86_64".into(),
            has_custom_resolution: false,
        };
        // "natives-macos" is declared in the `natives` map but has no
        // matching entry under `classifiers` in this fixture - must not
        // panic, just yield nothing for it.
        assert!(info.native_artifacts(&macos_ctx).is_empty());
    }

    #[test]
    fn jvm_arguments_resolve_conditional_entries_by_platform() {
        let info: VersionInfo = serde_json::from_str(modern_version_json()).unwrap();
        let args = info.jvm_arguments(&ctx());
        assert!(args.contains(&"-Djava.library.path=${natives_directory}".to_string()));
        assert!(!args.contains(&"-XstartOnFirstThread".to_string())); // macOS-only, we're "windows"

        let macos_ctx = RuleContext {
            os_name: "osx".into(),
            os_arch: "x86_64".into(),
            has_custom_resolution: false,
        };
        let mac_args = info.jvm_arguments(&macos_ctx);
        assert!(mac_args.contains(&"-XstartOnFirstThread".to_string()));
    }

    #[test]
    fn game_arguments_include_conditional_multi_value_entries_when_enabled() {
        let info: VersionInfo = serde_json::from_str(modern_version_json()).unwrap();
        let without_custom_res = info.game_arguments(&ctx()).unwrap();
        assert!(!without_custom_res.contains(&"--width".to_string()));

        let with_custom_res_ctx = RuleContext {
            has_custom_resolution: true,
            ..ctx()
        };
        let with_custom_res = info.game_arguments(&with_custom_res_ctx).unwrap();
        assert!(with_custom_res.contains(&"--width".to_string()));
        assert!(with_custom_res.contains(&"${resolution_width}".to_string()));
    }

    #[test]
    fn legacy_version_falls_back_to_minecraft_arguments_string() {
        let info: VersionInfo = serde_json::from_str(legacy_version_json()).unwrap();
        let args = info.game_arguments(&ctx()).unwrap();
        assert_eq!(
            args,
            vec![
                "--username",
                "${auth_player_name}",
                "--version",
                "${version_name}",
                "--uuid",
                "${auth_uuid}"
            ]
        );
    }

    #[test]
    fn legacy_version_gets_sane_default_jvm_arguments() {
        let info: VersionInfo = serde_json::from_str(legacy_version_json()).unwrap();
        let args = info.jvm_arguments(&ctx());
        assert!(args.contains(&"${classpath}".to_string()));
    }

    #[test]
    fn legacy_version_has_no_java_requirement_and_that_is_fine() {
        let info: VersionInfo = serde_json::from_str(legacy_version_json()).unwrap();
        assert!(info.java_version.is_none());
    }
}
