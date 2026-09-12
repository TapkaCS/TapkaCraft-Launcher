/**
 * Typed wrappers around the modpack Tauri commands
 * (`src-tauri/src/commands/modpacks.rs`). Browsing reuses the same
 * `SearchResponse`/`ModrinthVersion` shapes as `lib/api/modrinth.ts` -
 * Modrinth hosts modpacks in the same catalog as mods, just a different
 * `project_type`. Importing either source converges on the same
 * `InstanceMeta` result, since both create a fresh instance.
 */
import { invoke } from "@tauri-apps/api/core";

import type { InstanceMeta } from "@/types/instance";
import type { ModrinthVersion, SearchResponse } from "@/types/modrinth";

export function searchModpacks(
  query: string,
  loader: string,
  gameVersion: string,
): Promise<SearchResponse> {
  return invoke<SearchResponse>("search_modpacks", { query, loader, gameVersion });
}

export function listModpackVersions(
  projectId: string,
  loader: string,
  gameVersion: string,
): Promise<ModrinthVersion[]> {
  return invoke<ModrinthVersion[]>("list_modpack_versions", { projectId, loader, gameVersion });
}

export function importModpackFile(path: string, concurrency: number): Promise<InstanceMeta> {
  return invoke<InstanceMeta>("import_modpack_file", { path, concurrency });
}

export function installModpackFromModrinth(
  version: ModrinthVersion,
  concurrency: number,
): Promise<InstanceMeta> {
  return invoke<InstanceMeta>("install_modpack_from_modrinth", { version, concurrency });
}
