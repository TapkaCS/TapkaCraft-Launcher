//! Finds candidate Java installations (`PATH`, `JAVA_HOME`, known install
//! locations, the Windows registry, and TapkaCraft's own managed
//! runtimes), then verifies each one by actually running it - never
//! trusting a folder name alone.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::runtime::{parse_version_output, DetectedRuntime};

const JAVA_EXE: &str = if cfg!(windows) { "java.exe" } else { "java" };

/// Every place a real Java installation might be, deduplicated by
/// canonical path. Doesn't verify anything yet - `detect_all` does that.
pub fn candidate_java_executables(managed_runtimes_dir: &Path) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();

    let mut push = |path: PathBuf| {
        if !path.is_file() {
            return;
        }
        let key = path.canonicalize().unwrap_or(path.clone());
        if seen.insert(key) {
            candidates.push(path);
        }
    };

    if let Some(java_home) = std::env::var_os("JAVA_HOME") {
        push(Path::new(&java_home).join("bin").join(JAVA_EXE));
    }

    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            push(dir.join(JAVA_EXE));
        }
    }

    for dir in platform_install_dirs() {
        for entry in list_dir(&dir) {
            push(entry.join("bin").join(JAVA_EXE));
        }
    }

    // TapkaCraft-managed runtimes (runtimes/java21/, runtimes/java8/, ...)
    // downloaded by a later phase - detected the same way as anything else
    // once they exist, never assumed present.
    for entry in list_dir(managed_runtimes_dir) {
        push(entry.join("bin").join(JAVA_EXE));
    }

    candidates
}

fn list_dir(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(windows)]
fn platform_install_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for env_var in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(base) = std::env::var_os(env_var) {
            let base = PathBuf::from(base);
            for vendor_dir in [
                "Java",
                "Eclipse Adoptium",
                "Eclipse Foundation",
                "Microsoft",
                "Zulu",
                "Amazon Corretto",
            ] {
                dirs.push(base.join(vendor_dir));
            }
        }
    }
    dirs.extend(registry_java_dirs());
    dirs
}

#[cfg(not(windows))]
fn platform_install_dirs() -> Vec<PathBuf> {
    // Not the target platform, but keeping this real (not a stub) means
    // JavaManager is actually exercisable in this dev/CI environment too.
    vec![
        PathBuf::from("/usr/lib/jvm"),
        PathBuf::from("/usr/lib64/jvm"),
        PathBuf::from("/opt/java"),
        PathBuf::from("/Library/Java/JavaVirtualMachines"),
    ]
}

/// Reads `HKLM\SOFTWARE\JavaSoft\*` and `HKLM\SOFTWARE\Eclipse Adoptium\*`
/// for `JavaHome` values. Missing/unreadable keys are skipped, not errors -
/// most machines will only have some of these vendors installed, if any.
#[cfg(windows)]
fn registry_java_dirs() -> Vec<PathBuf> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

    const ROOTS: &[&str] = &[
        r"SOFTWARE\JavaSoft\JDK",
        r"SOFTWARE\JavaSoft\JRE",
        r"SOFTWARE\JavaSoft\Java Development Kit",
        r"SOFTWARE\JavaSoft\Java Runtime Environment",
        r"SOFTWARE\Eclipse Adoptium\JDK",
        r"SOFTWARE\Eclipse Adoptium\JRE",
        r"SOFTWARE\Microsoft\JDK",
    ];

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let mut dirs = Vec::new();
    for root in ROOTS {
        let Ok(root_key) = hklm.open_subkey(root) else {
            continue;
        };
        for version_name in root_key.enum_keys().filter_map(Result::ok) {
            let Ok(version_key) = root_key.open_subkey(&version_name) else {
                continue;
            };
            if let Ok(java_home) = version_key.get_value::<String, _>("JavaHome") {
                dirs.push(PathBuf::from(java_home));
            }
        }
    }
    dirs
}

/// Runs `<java> -version` and parses the result. `None` (not an error) for
/// a path that doesn't actually behave like a Java executable - a stale
/// registry entry pointing at a removed install is expected, not
/// exceptional.
pub fn verify(java_path: &Path) -> Option<DetectedRuntime> {
    let output = Command::new(java_path).arg("-version").output().ok()?;
    // Every JDK/JRE version has printed this to stderr; a few very new
    // builds also echo it to stdout, so checking both is the reliable path.
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    parse_version_output(java_path, &combined)
}

/// Detects and verifies every candidate. Order is not significant to
/// callers - `best_match` does the actual selection.
pub fn detect_all(managed_runtimes_dir: &Path) -> Vec<DetectedRuntime> {
    candidate_java_executables(managed_runtimes_dir)
        .into_iter()
        .filter_map(|path| verify(&path))
        .collect()
}

/// Picks the best runtime for a required major version: an exact match
/// first, otherwise the closest newer one (never an older one - Minecraft
/// does not run on a Java older than it declares it needs).
pub fn best_match(runtimes: &[DetectedRuntime], required_major: u32) -> Option<&DetectedRuntime> {
    if let Some(exact) = runtimes.iter().find(|r| r.major_version == required_major) {
        return Some(exact);
    }
    runtimes
        .iter()
        .filter(|r| r.major_version > required_major)
        .min_by_key(|r| r.major_version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::java::runtime::DetectedRuntime;

    fn runtime(major: u32) -> DetectedRuntime {
        DetectedRuntime {
            java_path: PathBuf::from(format!("/fake/java{major}")),
            major_version: major,
            version_string: format!("{major}.0.0"),
            is_64_bit: true,
            vendor: None,
        }
    }

    #[test]
    fn best_match_prefers_an_exact_major_version() {
        let runtimes = vec![runtime(8), runtime(17), runtime(21)];
        assert_eq!(best_match(&runtimes, 17).unwrap().major_version, 17);
    }

    #[test]
    fn best_match_falls_back_to_the_closest_newer_version() {
        let runtimes = vec![runtime(8), runtime(21)];
        assert_eq!(best_match(&runtimes, 17).unwrap().major_version, 21);
    }

    #[test]
    fn best_match_never_picks_an_older_runtime_than_required() {
        let runtimes = vec![runtime(8), runtime(11)];
        assert!(best_match(&runtimes, 17).is_none());
    }

    #[test]
    fn best_match_returns_none_when_no_runtimes_are_available() {
        assert!(best_match(&[], 17).is_none());
    }

    #[test]
    fn verify_returns_none_for_a_nonexistent_path_instead_of_erroring() {
        assert!(verify(Path::new("/definitely/not/a/real/java/binary")).is_none());
    }

    #[test]
    fn verify_works_against_this_machine_s_actual_java_if_one_is_installed() {
        // Best-effort real integration check: if this sandbox happens to
        // have a JDK on PATH, confirm we can actually detect+verify it
        // end to end, not just against synthetic fixtures.
        let Ok(path_var) = std::env::var("PATH") else {
            return;
        };
        let Some(java_dir) =
            std::env::split_paths(&path_var).find(|dir| dir.join(JAVA_EXE).is_file())
        else {
            return; // no system Java in this environment - nothing to assert
        };
        let runtime = verify(&java_dir.join(JAVA_EXE));
        assert!(
            runtime.is_some(),
            "found a `java` on PATH but couldn't verify it"
        );
    }

    #[test]
    fn candidate_list_has_no_duplicate_canonical_paths() {
        let dir = tempfile::tempdir().unwrap();
        let managed = dir.path().join("runtimes");
        let candidates = candidate_java_executables(&managed);
        let mut seen = HashSet::new();
        for c in &candidates {
            let canon = c.canonicalize().unwrap_or_else(|_| c.clone());
            assert!(seen.insert(canon), "duplicate candidate: {c:?}");
        }
    }
}
