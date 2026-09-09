/**
 * Mirrors `core::modrinth` in `src-tauri/src/core/modrinth/`. The search/
 * version/file/dependency shapes below use Modrinth's own snake_case field
 * names verbatim (the Rust structs behind them have to stay snake_case to
 * deserialize Modrinth's real API responses, so they cross the Tauri IPC
 * boundary as-is too) - the one place in this codebase that isn't
 * camelCase. `InstalledMod`/`InstalledMods` are this launcher's own data
 * and are camelCase like everything else.
 */

export interface SearchHit {
  project_id: string;
  slug: string;
  title: string;
  description: string;
  icon_url: string | null;
  downloads: number;
  categories: string[];
}

export interface SearchResponse {
  hits: SearchHit[];
  total_hits: number;
}

export interface ModrinthDependency {
  project_id: string | null;
  version_id: string | null;
  dependency_type: "required" | "optional" | "incompatible" | "embedded";
}

export interface ModrinthFileHashes {
  sha1: string;
}

export interface ModrinthFile {
  url: string;
  filename: string;
  primary: boolean;
  hashes: ModrinthFileHashes;
}

export interface ModrinthVersion {
  id: string;
  project_id: string;
  name: string;
  version_number: string;
  game_versions: string[];
  loaders: string[];
  dependencies: ModrinthDependency[];
  files: ModrinthFile[];
}

export interface InstalledMod {
  projectId: string;
  versionId: string;
  title: string;
  fileName: string;
}

export interface InstalledMods {
  mods: Record<string, InstalledMod>;
}

export interface InstallModOutcome {
  installed: InstalledMod[];
  skippedDependencies: string[];
}
