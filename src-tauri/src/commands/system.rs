//! Tauri IPC wrapper around `core::system`.

use crate::core::system::{self, GpuInfo, MemorySuggestion};

/// Suggested JVM memory range for a new profile, sized from this machine's
/// total RAM.
#[tauri::command]
pub fn get_memory_suggestion() -> MemorySuggestion {
    system::suggest_memory()
}

/// Best-effort GPU inventory (diagnostic display only - nothing in the
/// launcher changes behavior based on it yet). Empty on a machine/platform
/// this can't inspect, not an error.
#[tauri::command]
pub fn get_gpu_info() -> Vec<GpuInfo> {
    system::detect_gpus()
}
