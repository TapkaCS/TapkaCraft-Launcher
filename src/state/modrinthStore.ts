import { create } from "zustand";

import {
  installContent,
  listContentVersions,
  listInstalledContent,
  searchContent,
} from "@/lib/api/modrinth";
import { listenToInstallProgress } from "@/lib/api/versions";
import { isTauri } from "@/lib/tauri";
import { applyInstallProgress, type InstallProgress } from "@/state/installStore";
import { useSettingsStore } from "@/state/settingsStore";
import type { ContentKind, InstalledContentList, SearchHit } from "@/types/modrinth";

interface ModrinthState {
  contentKind: ContentKind;
  /** Switching category clears the previous category's results/installed list - they're a different catalog. */
  setContentKind: (kind: ContentKind) => void;

  query: string;
  setQuery: (query: string) => void;

  /**
   * Genre/theme tags (e.g. "adventure", "optimization") to additionally
   * filter by, OR'd together - always tags a real search hit already came
   * back with (see the category chips on each result), never a hardcoded
   * list, so there's no risk of a guessed tag silently matching nothing.
   */
  categories: string[];
  toggleCategory: (category: string) => void;

  results: SearchHit[];
  searchStatus: "idle" | "loading" | "error";
  searchError: string | null;
  search: (loader: string, gameVersion: string) => Promise<void>;

  installedContent: InstalledContentList | null;
  loadInstalledContent: (instanceId: string) => Promise<void>;

  installingProjectId: string | null;
  installProgress: InstallProgress | null;
  installError: string | null;
  skippedDependencies: string[];
  install: (
    instanceId: string,
    projectId: string,
    loader: string,
    gameVersion: string,
  ) => Promise<void>;
}

function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export const useModrinthStore = create<ModrinthState>((set, get) => ({
  contentKind: "mod",
  setContentKind: (kind) =>
    set({
      contentKind: kind,
      categories: [],
      results: [],
      searchStatus: "idle",
      searchError: null,
      installedContent: null,
      installError: null,
      skippedDependencies: [],
    }),

  query: "",
  setQuery: (query) => set({ query }),

  categories: [],
  toggleCategory: (category) =>
    set((state) => ({
      categories: state.categories.includes(category)
        ? state.categories.filter((existing) => existing !== category)
        : [...state.categories, category],
    })),

  results: [],
  searchStatus: "idle",
  searchError: null,
  search: async (loader, gameVersion) => {
    if (!isTauri) {
      set({ searchStatus: "error", searchError: "Searching Modrinth requires the desktop app." });
      return;
    }
    set({ searchStatus: "loading", searchError: null });
    try {
      const response = await searchContent(
        get().query,
        get().contentKind,
        loader,
        gameVersion,
        get().categories,
      );
      set({ results: response.hits, searchStatus: "idle" });
    } catch (err) {
      set({ searchStatus: "error", searchError: errorMessage(err) });
    }
  },

  installedContent: null,
  loadInstalledContent: async (instanceId) => {
    if (!isTauri) return;
    try {
      const installed = await listInstalledContent(instanceId, get().contentKind);
      set({ installedContent: installed });
    } catch {
      // Best-effort UI hint only - a failure here shouldn't block browsing.
      set({ installedContent: null });
    }
  },

  installingProjectId: null,
  installProgress: null,
  installError: null,
  skippedDependencies: [],
  install: async (instanceId, projectId, loader, gameVersion) => {
    if (get().installingProjectId) return; // one install at a time
    if (!isTauri) {
      set({ installError: "Installing requires the desktop app." });
      return;
    }

    const contentKind = get().contentKind;
    set({
      installingProjectId: projectId,
      installProgress: null,
      installError: null,
      skippedDependencies: [],
    });

    const unlisten = await listenToInstallProgress((event) => {
      set((state) => ({ installProgress: applyInstallProgress(state.installProgress, event) }));
    });

    try {
      const versions = await listContentVersions(projectId, contentKind, loader, gameVersion);
      const newest = versions[0];
      if (!newest) {
        throw new Error("No version of this is compatible with the selected profile.");
      }
      const concurrency = useSettingsStore.getState().settings.downloads.concurrentDownloads;
      const outcome = await installContent(instanceId, contentKind, newest, concurrency);
      set({ installingProjectId: null, skippedDependencies: outcome.skippedDependencies });
      await get().loadInstalledContent(instanceId);
    } catch (err) {
      set({ installingProjectId: null, installError: errorMessage(err) });
    } finally {
      unlisten();
    }
  },
}));
