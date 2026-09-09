/**
 * Mirrors the Rust types Tauri serializes for version/install commands:
 * `VersionSummary` (`src-tauri/src/commands/versions.rs`) and
 * `DownloadEvent` (`src-tauri/src/core/downloads/mod.rs`), emitted as
 * `install://progress` events during `install_instance`.
 */

export interface VersionSummary {
  id: string;
  releaseTime: string;
}

export type InstallProgressEvent =
  | { type: "started"; totalFiles: number }
  | { type: "fileProgress"; label: string; bytesDownloaded: number; totalBytes: number | null }
  | { type: "fileCompleted"; label: string; cached: boolean }
  | { type: "fileFailed"; label: string; error: string }
  | { type: "finished"; succeeded: number; failed: number };
