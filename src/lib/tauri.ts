import { invoke } from "@tauri-apps/api/core";

/**
 * True when running inside the Tauri webview. False when the frontend is
 * previewed standalone via `vite dev`/`vite preview` in a regular browser,
 * which has no Tauri IPC bridge - useful for UI development without
 * spinning up the Rust side every time.
 */
export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/**
 * Calls the one real Tauri command Phase 1 wires up. Falls back to a
 * static "dev" string outside the Tauri webview instead of throwing, so
 * the UI stays usable in a plain-browser preview.
 */
export async function getLauncherVersion(): Promise<string> {
  if (!isTauri) return "dev";
  return invoke<string>("get_launcher_version");
}
