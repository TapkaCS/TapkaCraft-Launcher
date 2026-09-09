//! A verified Java runtime and the pure string-parsing logic that turns
//! `java -version`'s output into structured data.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedRuntime {
    /// Path to the `java`/`java.exe` binary that was actually executed to
    /// verify this runtime - the source of truth, never a guess from a
    /// folder name.
    pub java_path: PathBuf,
    pub major_version: u32,
    pub version_string: String,
    pub is_64_bit: bool,
    pub vendor: Option<String>,
}

impl DetectedRuntime {
    /// The executable to actually launch Minecraft with: `javaw.exe`
    /// alongside `java.exe` on Windows, when present, so no console window
    /// appears. Falls back to `java_path` everywhere else (and if
    /// `javaw.exe` isn't there for some reason).
    pub fn launch_executable(&self) -> PathBuf {
        if cfg!(windows) {
            if let Some(file_name) = self.java_path.file_name().and_then(|n| n.to_str()) {
                if file_name.eq_ignore_ascii_case("java.exe") {
                    let candidate = self.java_path.with_file_name("javaw.exe");
                    if candidate.is_file() {
                        return candidate;
                    }
                }
            }
        }
        self.java_path.clone()
    }
}

/// Parses the major version out of a `java -version`-style version string.
/// Handles both the legacy `1.8.0_431` scheme (Java 8 and earlier, where
/// the real major version is the *second* component) and the modern
/// `21.0.5` / `17` scheme (Java 9+, where it's the first).
pub fn parse_major_version(version_string: &str) -> Option<u32> {
    let mut parts = version_string.split(['.', '-', '+']);
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 {
        parts.next()?.parse().ok()
    } else {
        Some(first)
    }
}

/// Extracts the quoted (or bare, on some newer builds) version string from
/// the first line of `java -version` output, e.g. `openjdk version "21.0.5" ...`
/// or `openjdk 21.0.5 2024-10-15`.
pub fn extract_version_string(first_line: &str) -> Option<String> {
    if let Some(start) = first_line.find('"') {
        let rest = &first_line[start + 1..];
        let end = rest.find('"')?;
        return Some(rest[..end].to_string());
    }
    // No quotes: take the second whitespace-separated token (after
    // "openjdk"/"java") if it looks like a version number.
    let mut words = first_line.split_whitespace();
    words.next()?; // "openjdk" / "java"
    let candidate = words.next()?;
    if candidate.chars().next()?.is_ascii_digit() {
        Some(candidate.to_string())
    } else {
        None
    }
}

pub fn is_64_bit(full_output: &str) -> bool {
    full_output.contains("64-Bit") || full_output.contains("64-bit")
}

/// Best-effort vendor name from the runtime-environment line, e.g.
/// "OpenJDK Runtime Environment Temurin-21.0.5+11 ..." -> "Temurin".
/// `None` rather than a guess when nothing recognizable is found.
pub fn extract_vendor(full_output: &str) -> Option<String> {
    const KNOWN_VENDORS: &[&str] = &[
        "Temurin",
        "Zulu",
        "Corretto",
        "Microsoft",
        "GraalVM",
        "OpenJ9",
        "IBM",
        "Oracle",
    ];
    KNOWN_VENDORS
        .iter()
        .find(|vendor| full_output.contains(**vendor))
        .map(|v| v.to_string())
}

pub fn parse_version_output(java_path: &Path, output: &str) -> Option<DetectedRuntime> {
    // The actual version line is usually first, but a JVM that picks up
    // `JAVA_TOOL_OPTIONS`/`_JAVA_OPTIONS` from the environment prints a
    // "Picked up ..." diagnostic line before it - scan for the real one
    // instead of assuming it's line one.
    let version_string = output
        .lines()
        .find(|line| line.starts_with("openjdk") || line.starts_with("java"))
        .and_then(extract_version_string)?;
    let major_version = parse_major_version(&version_string)?;
    Some(DetectedRuntime {
        java_path: java_path.to_path_buf(),
        major_version,
        version_string,
        is_64_bit: is_64_bit(output),
        vendor: extract_vendor(output),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modern_major_version() {
        assert_eq!(parse_major_version("21.0.5"), Some(21));
        assert_eq!(parse_major_version("17"), Some(17));
        assert_eq!(parse_major_version("11.0.2"), Some(11));
    }

    #[test]
    fn parses_legacy_1_dot_x_major_version() {
        assert_eq!(parse_major_version("1.8.0_431"), Some(8));
        assert_eq!(parse_major_version("1.7.0_80"), Some(7));
    }

    #[test]
    fn extracts_quoted_version_string() {
        assert_eq!(
            extract_version_string(r#"openjdk version "21.0.5" 2024-10-15"#),
            Some("21.0.5".to_string())
        );
        assert_eq!(
            extract_version_string(r#"java version "1.8.0_431""#),
            Some("1.8.0_431".to_string())
        );
    }

    #[test]
    fn extracts_unquoted_version_string_from_newer_builds() {
        assert_eq!(
            extract_version_string("openjdk 21.0.5 2024-10-15"),
            Some("21.0.5".to_string())
        );
    }

    #[test]
    fn detects_64_bit_marker() {
        let output = "OpenJDK 64-Bit Server VM Temurin-21.0.5+11 (build 21.0.5+11, mixed mode)";
        assert!(is_64_bit(output));
        assert!(!is_64_bit(
            "OpenJDK Client VM (build 25.431-b10, mixed mode)"
        ));
    }

    #[test]
    fn extracts_known_vendor_names() {
        assert_eq!(
            extract_vendor("OpenJDK Runtime Environment Temurin-21.0.5+11"),
            Some("Temurin".to_string())
        );
        assert_eq!(
            extract_vendor("OpenJDK Runtime Environment Corretto-17.0.9.9.1"),
            Some("Corretto".to_string())
        );
        assert_eq!(extract_vendor("some completely unrecognized string"), None);
    }

    #[test]
    fn tolerates_a_java_tool_options_preamble_line_before_the_real_version_line() {
        // Real output captured from this sandbox, where JAVA_TOOL_OPTIONS is
        // set: the JVM prints a "Picked up ..." diagnostic to stderr first.
        let path = Path::new("/usr/bin/java");
        let output = "Picked up JAVA_TOOL_OPTIONS: -Djavax.net.ssl.trustStore=/root/.ccr/java-truststore.p12\n\
             openjdk version \"21.0.10\" 2026-01-20\n\
             OpenJDK Runtime Environment (build 21.0.10+7-Ubuntu-124.04)\n\
             OpenJDK 64-Bit Server VM (build 21.0.10+7-Ubuntu-124.04, mixed mode, sharing)\n";
        let runtime = parse_version_output(path, output).unwrap();
        assert_eq!(runtime.major_version, 21);
        assert_eq!(runtime.version_string, "21.0.10");
        assert!(runtime.is_64_bit);
    }

    #[test]
    fn parses_a_full_modern_temurin_output() {
        let path = Path::new("/usr/lib/jvm/temurin-21/bin/java");
        let output = "openjdk version \"21.0.5\" 2024-10-15\n\
             OpenJDK Runtime Environment Temurin-21.0.5+11 (build 21.0.5+11)\n\
             OpenJDK 64-Bit Server VM Temurin-21.0.5+11 (build 21.0.5+11, mixed mode, sharing)\n";
        let runtime = parse_version_output(path, output).unwrap();
        assert_eq!(runtime.major_version, 21);
        assert_eq!(runtime.version_string, "21.0.5");
        assert!(runtime.is_64_bit);
        assert_eq!(runtime.vendor, Some("Temurin".to_string()));
    }

    #[test]
    fn parses_a_full_legacy_java_8_output() {
        let path = Path::new(r"C:\Program Files\Java\jre1.8.0_431\bin\java.exe");
        let output = "java version \"1.8.0_431\"\n\
             Java(TM) SE Runtime Environment (build 1.8.0_431-b10)\n\
             Java HotSpot(TM) 64-Bit Server VM (build 25.431-b10, mixed mode)\n";
        let runtime = parse_version_output(path, output).unwrap();
        assert_eq!(runtime.major_version, 8);
        assert_eq!(runtime.version_string, "1.8.0_431");
        assert!(runtime.is_64_bit);
    }

    #[test]
    fn garbage_output_yields_none_instead_of_panicking() {
        let path = Path::new("/nonexistent/java");
        assert!(parse_version_output(path, "not java output at all").is_none());
        assert!(parse_version_output(path, "").is_none());
    }

    #[test]
    fn launch_executable_prefers_javaw_on_windows_when_present() {
        let dir = tempfile::tempdir().unwrap();
        let java_exe = dir.path().join("java.exe");
        let javaw_exe = dir.path().join("javaw.exe");
        std::fs::write(&java_exe, b"").unwrap();
        std::fs::write(&javaw_exe, b"").unwrap();

        let runtime = DetectedRuntime {
            java_path: java_exe.clone(),
            major_version: 21,
            version_string: "21.0.5".into(),
            is_64_bit: true,
            vendor: None,
        };

        if cfg!(windows) {
            assert_eq!(runtime.launch_executable(), javaw_exe);
        } else {
            assert_eq!(runtime.launch_executable(), java_exe);
        }
    }

    #[test]
    fn launch_executable_falls_back_to_java_when_javaw_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let java_exe = dir.path().join("java.exe");
        std::fs::write(&java_exe, b"").unwrap();

        let runtime = DetectedRuntime {
            java_path: java_exe.clone(),
            major_version: 21,
            version_string: "21.0.5".into(),
            is_64_bit: true,
            vendor: None,
        };
        assert_eq!(runtime.launch_executable(), java_exe);
    }
}
