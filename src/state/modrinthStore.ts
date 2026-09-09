import { create } from "zustand";

import { installMod, listInstalledMods, listModVersions, searchMods } from "@/lib/api/modrinth";
import { listenToInstallProgress } from "@/lib/api/versions";
import { isTauri } from "@/lib/tauri";
import { applyInstallProgress, type InstallProgress } from "@/state/installStore";
import { useSettingsStore } from "@/state/settingsStore";
import type { InstalledMods, SearchHit } from "@/types/modrinth";

interface ModrinthState {
  query: string;
  setQuery: (query: string) => void;

  results: SearchHit[];
  searchStatus: "idle" | "loading" | "error";
  searchError: string | null;
  search: (loader: string, gameVersion: string) => Promise<void>;

  installedMods: InstalledMods | null;
  loadInstalledMods: (instanceId: string) => Promise<void>;

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
  query: "",
  setQuery: (query) => set({ query }),

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
      const response = await searchMods(get().query, loader, gameVersion);
      set({ results: response.hits, searchStatus: "idle" });
    } catch (err) {
      set({ searchStatus: "error", searchError: errorMessage(err) });
    }
  },

  installedMods: null,
  loadInstalledMods: async (instanceId) => {
    if (!isTauri) return;
    try {
      const installed = await listInstalledMods(instanceId);
      set({ installedMods: installed });
    } catch {
      // Best-effort UI hint only - a failure here shouldn't block browsing.
      set({ installedMods: null });
    }
  },

  installingProjectId: null,
  installProgress: null,
  installError: null,
  skippedDependencies: [],
  install: async (instanceId, projectId, loader, gameVersion) => {
    if (get().installingProjectId) return; // one install at a time
    if (!isTauri) {
      set({ installError: "Installing mods requires the desktop app." });
      return;
    }

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
      const versions = await listModVersions(projectId, loader, gameVersion);
      const newest = versions[0];
      if (!newest) {
        throw new Error("No version of this mod is compatible with the selected profile.");
      }
      const concurrency = useSettingsStore.getState().settings.downloads.concurrentDownloads;
      const outcome = await installMod(instanceId, newest, concurrency);
      set({ installingProjectId: null, skippedDependencies: outcome.skippedDependencies });
      await get().loadInstalledMods(instanceId);
    } catch (err) {
      set({ installingProjectId: null, installError: errorMessage(err) });
    } finally {
      unlisten();
    }
  },
}));
