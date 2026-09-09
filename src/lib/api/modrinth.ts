/**
 * Typed wrappers around the Modrinth Tauri commands
 * (`src-tauri/src/commands/modrinth.rs`).
 */
import { invoke } from "@tauri-apps/api/core";

import type {
  InstallModOutcome,
  InstalledMods,
  ModrinthVersion,
  SearchResponse,
} from "@/types/modrinth";

export function searchMods(
  query: string,
  loader: string,
  gameVersion: string,
): Promise<SearchResponse> {
  return invoke<SearchResponse>("search_mods", { query, loader, gameVersion });
}

export function listModVersions(
  projectId: string,
  loader: string,
  gameVersion: string,
): Promise<ModrinthVersion[]> {
  return invoke<ModrinthVersion[]>("list_mod_versions", { projectId, loader, gameVersion });
}

export function listInstalledMods(id: string): Promise<InstalledMods> {
  return invoke<InstalledMods>("list_installed_mods", { id });
}

export function installMod(
  id: string,
  version: ModrinthVersion,
  concurrency: number,
): Promise<InstallModOutcome> {
  return invoke<InstallModOutcome>("install_mod", { id, version, concurrency });
}
