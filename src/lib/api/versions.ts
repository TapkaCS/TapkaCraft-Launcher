/**
 * Typed wrappers around the version/install Tauri commands
 * (`src-tauri/src/commands/versions.rs`). Always talk to the real Rust
 * backend, same posture as `lib/api/instances.ts` - callers decide whether
 * to use these or a browser-preview fallback.
 */
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { InstallProgressEvent, VersionSummary } from "@/types/version";

const INSTALL_PROGRESS_EVENT = "install://progress";

export function getVersionManifest(): Promise<VersionSummary[]> {
  return invoke<VersionSummary[]>("get_version_manifest");
}

export function installInstance(id: string, concurrency: number): Promise<void> {
  return invoke<void>("install_instance", { id, concurrency });
}

/** Subscribes to install progress events; call the returned `UnlistenFn` when done. */
export function listenToInstallProgress(
  callback: (event: InstallProgressEvent) => void,
): Promise<UnlistenFn> {
  return listen<InstallProgressEvent>(INSTALL_PROGRESS_EVENT, (event) => callback(event.payload));
}
