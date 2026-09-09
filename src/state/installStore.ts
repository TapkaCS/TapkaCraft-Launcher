import { create } from "zustand";

import { installInstance, listenToInstallProgress } from "@/lib/api/versions";
import { isTauri } from "@/lib/tauri";
import { useSettingsStore } from "@/state/settingsStore";
import type { InstallProgressEvent } from "@/types/version";

export type InstallPhase = "idle" | "installing" | "done" | "error";

export interface InstallProgress {
  totalFiles: number;
  completedFiles: number;
  failedFiles: number;
  currentLabel: string | null;
}

interface InstallState {
  installingInstanceId: string | null;
  phase: InstallPhase;
  progress: InstallProgress | null;
  error: string | null;
  /**
   * Instance ids installed at least once this session. Ephemeral by design:
   * it is not persisted and not re-checked on startup, so after a restart
   * the button reads "Install" again even for an already-installed
   * instance - clicking it re-verifies existing files almost instantly
   * rather than re-downloading them, since `ensure_installed` skips
   * anything already correct on disk.
   */
  installedThisSession: Set<string>;
  install: (id: string) => Promise<void>;
}

const EMPTY_PROGRESS: InstallProgress = {
  totalFiles: 0,
  completedFiles: 0,
  failedFiles: 0,
  currentLabel: null,
};

function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

/** Pure reducer for one `DownloadEvent` - split out so it's testable without a Tauri event bus. */
export function applyInstallProgress(
  current: InstallProgress | null,
  event: InstallProgressEvent,
): InstallProgress {
  const base = current ?? EMPTY_PROGRESS;
  switch (event.type) {
    case "started":
      return { ...EMPTY_PROGRESS, totalFiles: event.totalFiles };
    case "fileProgress":
      return { ...base, currentLabel: event.label };
    case "fileCompleted":
      return { ...base, completedFiles: base.completedFiles + 1, currentLabel: event.label };
    case "fileFailed":
      return { ...base, failedFiles: base.failedFiles + 1, currentLabel: event.label };
    case "finished":
      return { ...base, currentLabel: null };
  }
}

export const useInstallStore = create<InstallState>((set, get) => ({
  installingInstanceId: null,
  phase: "idle",
  progress: null,
  error: null,
  installedThisSession: new Set(),

  install: async (id) => {
    if (get().installingInstanceId) return; // one install at a time
    if (!isTauri) {
      set({
        phase: "error",
        error: "Installing requires the desktop app, not the browser preview.",
      });
      return;
    }

    set({
      installingInstanceId: id,
      phase: "installing",
      progress: { ...EMPTY_PROGRESS },
      error: null,
    });

    const unlisten = await listenToInstallProgress((event) => {
      set((state) => ({ progress: applyInstallProgress(state.progress, event) }));
    });

    try {
      const concurrency = useSettingsStore.getState().settings.downloads.concurrentDownloads;
      await installInstance(id, concurrency);
      set((state) => {
        const installedThisSession = new Set(state.installedThisSession);
        installedThisSession.add(id);
        return { phase: "done", installingInstanceId: null, installedThisSession };
      });
    } catch (err) {
      set({ phase: "error", installingInstanceId: null, error: errorMessage(err) });
    } finally {
      unlisten();
    }
  },
}));
