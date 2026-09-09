//! Tauri IPC wrapper around `core::java::detect`.

use tauri::State;

use crate::core::java::detect;
use crate::core::java::runtime::DetectedRuntime;
use crate::core::paths::AppPaths;

/// Every Java runtime this machine has that TapkaCraft can detect and
/// verify by actually running it - PATH, `JAVA_HOME`, known install
/// locations, the Windows registry, and TapkaCraft's own managed runtimes
/// once any exist. Runs `java -version` for each candidate, so this is a
/// blocking call; Tauri already dispatches synchronous commands off the
/// main thread.
#[tauri::command]
pub fn detect_java_runtimes(app_paths: State<AppPaths>) -> Vec<DetectedRuntime> {
    detect::detect_all(&app_paths.runtimes_dir())
}
