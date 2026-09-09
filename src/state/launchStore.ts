import { create } from "zustand";

import { launchInstance, listenToLaunchEvents } from "@/lib/api/launch";
import { listenToInstallProgress } from "@/lib/api/versions";
import { isTauri } from "@/lib/tauri";
import { applyInstallProgress, type InstallProgress } from "@/state/installStore";
import { useSettingsStore } from "@/state/settingsStore";

export type LaunchPhase = "idle" | "preparing" | "running" | "error";

const MAX_LOG_LINES = 500;

interface LaunchState {
  launchingInstanceId: string | null;
  phase: LaunchPhase;
  /**
   * Progress for the install `launch_instance` runs before spawning the
   * game - reuses the exact same `install://progress` event stream and
   * reducer the standalone Install button uses, so a Play-triggered
   * install looks and behaves identically to an explicit one.
   */
  installProgress: InstallProgress | null;
  logLines: string[];
  error: string | null;
  exitCode: number | null;

  launch: (id: string) => Promise<void>;
}

function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export const useLaunchStore = create<LaunchState>((set, get) => ({
  launchingInstanceId: null,
  phase: "idle",
  installProgress: null,
  logLines: [],
  error: null,
  exitCode: null,

  launch: async (id) => {
    if (get().launchingInstanceId) return; // one launch at a time
    if (!isTauri) {
      set({
        phase: "error",
        error: "Launching requires the desktop app, not the browser preview.",
      });
      return;
    }

    set({
      launchingInstanceId: id,
      phase: "preparing",
      installProgress: null,
      logLines: [],
      error: null,
      exitCode: null,
    });

    const unlistenInstall = await listenToInstallProgress((event) => {
      set((state) => ({ installProgress: applyInstallProgress(state.installProgress, event) }));
    });
    const unlistenLaunch = await listenToLaunchEvents((event) => {
      if (event.type === "started") {
        set({ phase: "running" });
      } else if (event.type === "output") {
        set((state) => ({ logLines: [...state.logLines, event.line].slice(-MAX_LOG_LINES) }));
      }
    });

    try {
      const concurrency = useSettingsStore.getState().settings.downloads.concurrentDownloads;
      const exitCode = await launchInstance(id, concurrency);
      set({ phase: "idle", launchingInstanceId: null, exitCode });
    } catch (err) {
      set({ phase: "error", launchingInstanceId: null, error: errorMessage(err) });
    } finally {
      unlistenInstall();
      unlistenLaunch();
    }
  },
}));
