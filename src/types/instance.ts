/**
 * Mirrors `InstanceMeta` in `src-tauri/src/core/instances/model.rs` and the
 * `instance.json` schema from the project spec's "INSTANCE METADATA"
 * section. Both sides describe the same on-disk document; keep them in
 * sync by hand until a shared schema generator exists.
 */

export type LoaderKind = "vanilla" | "fabric" | "quilt" | "forge" | "neoforge";

export interface LoaderConfig {
  type: LoaderKind;
  version: string;
}

export interface JavaInstanceSettings {
  memoryMinMb: number;
  memoryMaxMb: number;
}

export interface InstanceMeta {
  id: string;
  name: string;
  minecraftVersion: string;
  loader: LoaderConfig;
  java: JavaInstanceSettings;
  icon?: string;
  favorite: boolean;
  /** Total accumulated playtime. 0 until the launch engine exists (Phase 3). */
  playtimeSeconds: number;
  launchCount: number;
  /** ISO-8601 timestamp of the last successful launch, if any. */
  lastPlayed?: string;
}

export const LOADER_LABELS: Record<LoaderKind, string> = {
  vanilla: "Vanilla",
  fabric: "Fabric",
  quilt: "Quilt",
  forge: "Forge",
  neoforge: "NeoForge",
};
