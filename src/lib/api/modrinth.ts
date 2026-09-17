/**
 * Typed wrappers around the Modrinth Tauri commands
 * (`src-tauri/src/commands/modrinth.rs`).
 */
import { invoke } from "@tauri-apps/api/core";

import type {
  ContentKind,
  InstallContentOutcome,
  InstalledContentList,
  ModrinthVersion,
  SearchResponse,
} from "@/types/modrinth";

export function searchContent(
  query: string,
  contentKind: ContentKind,
  loader: string,
  gameVersion: string,
  categories: string[],
): Promise<SearchResponse> {
  return invoke<SearchResponse>("search_content", {
    query,
    contentKind,
    loader,
    gameVersion,
    categories,
  });
}

export function listContentVersions(
  projectId: string,
  contentKind: ContentKind,
  loader: string,
  gameVersion: string,
): Promise<ModrinthVersion[]> {
  return invoke<ModrinthVersion[]>("list_content_versions", {
    projectId,
    contentKind,
    loader,
    gameVersion,
  });
}

export function listInstalledContent(
  id: string,
  contentKind: ContentKind,
): Promise<InstalledContentList> {
  return invoke<InstalledContentList>("list_installed_content", { id, contentKind });
}

export function installContent(
  id: string,
  contentKind: ContentKind,
  version: ModrinthVersion,
  concurrency: number,
): Promise<InstallContentOutcome> {
  return invoke<InstallContentOutcome>("install_content", {
    id,
    contentKind,
    version,
    concurrency,
  });
}
