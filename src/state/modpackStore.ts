import { open } from "@tauri-apps/plugin-dialog";
import { create } from "zustand";

import {
  importModpackFile,
  installModpackFromModrinth,
  listModpackVersions,
  searchModpacks,
} from "@/lib/api/modpacks";
import { listenToInstallProgress } from "@/lib/api/versions";
import { isTauri } from "@/lib/tauri";
import { applyInstallProgress, type InstallProgress } from "@/state/installStore";
import { useInstanceStore } from "@/state/instanceStore";
import { useSettingsStore } from "@/state/settingsStore";
import { useUiStore } from "@/state/uiStore";
import type { SearchHit } from "@/types/modrinth";

interface ModpackState {
  query: string;
  setQuery: (query: string) => void;

  results: SearchHit[];
  searchStatus: "idle" | "loading" | "error";
  searchError: string | null;
  search: (loader: string, gameVersion: string) => Promise<void>;

  installingProjectId: string | null;
  isImportingFile: boolean;
  installProgress: InstallProgress | null;
  installError: string | null;
  install: (projectId: string, loader: string, gameVersion: string) => Promise<void>;
  importFromFile: () => Promise<void>;
}

function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

/**
 * After a modpack install (either source) finishes, the interesting result
 * isn't more search state - it's a brand new instance the user almost
 * certainly wants to see and play, so this jumps them straight to it
 * instead of leaving them on the Modpacks tab looking at search results.
 */
async function goToNewInstance(id: string) {
  await useInstanceStore.getState().loadInstances();
  useInstanceStore.getState().selectInstance(id);
  useUiStore.getState().setActiveTab("play");
}

export const useModpackStore = create<ModpackState>((set, get) => ({
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
      const response = await searchModpacks(get().query, loader, gameVersion);
      set({ results: response.hits, searchStatus: "idle" });
    } catch (err) {
      set({ searchStatus: "error", searchError: errorMessage(err) });
    }
  },

  installingProjectId: null,
  isImportingFile: false,
  installProgress: null,
  installError: null,

  install: async (projectId, loader, gameVersion) => {
    if (get().installingProjectId || get().isImportingFile) return;
    if (!isTauri) {
      set({ installError: "Installing modpacks requires the desktop app." });
      return;
    }

    set({ installingProjectId: projectId, installProgress: null, installError: null });
    const unlisten = await listenToInstallProgress((event) => {
      set((state) => ({ installProgress: applyInstallProgress(state.installProgress, event) }));
    });

    try {
      const versions = await listModpackVersions(projectId, loader, gameVersion);
      const newest = versions[0];
      if (!newest) {
        throw new Error("No version of this modpack matches the selected filters.");
      }
      const concurrency = useSettingsStore.getState().settings.downloads.concurrentDownloads;
      const instance = await installModpackFromModrinth(newest, concurrency);
      await goToNewInstance(instance.id);
    } catch (err) {
      set({ installError: errorMessage(err) });
    } finally {
      set({ installingProjectId: null });
      unlisten();
    }
  },

  importFromFile: async () => {
    if (get().installingProjectId || get().isImportingFile) return;
    if (!isTauri) {
      set({ installError: "Importing a modpack file requires the desktop app." });
      return;
    }

    const selected = await open({
      multiple: false,
      filters: [{ name: "Modrinth Modpack", extensions: ["mrpack"] }],
    });
    if (!selected || Array.isArray(selected)) return; // user cancelled the dialog

    set({ isImportingFile: true, installProgress: null, installError: null });
    const unlisten = await listenToInstallProgress((event) => {
      set((state) => ({ installProgress: applyInstallProgress(state.installProgress, event) }));
    });

    try {
      const concurrency = useSettingsStore.getState().settings.downloads.concurrentDownloads;
      const instance = await importModpackFile(selected, concurrency);
      await goToNewInstance(instance.id);
    } catch (err) {
      set({ installError: errorMessage(err) });
    } finally {
      set({ isImportingFile: false });
      unlisten();
    }
  },
}));
