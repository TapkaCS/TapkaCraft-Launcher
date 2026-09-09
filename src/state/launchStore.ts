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
  /**
   * `launch_instance` runs at most two download batches back to back on
   * this same event stream - the profile's Minecraft files, then (only if
   * no compatible Java was found) a Java runtime - each starting with its
   * own `started` event. Counting those tells the two apart so the label
   * can say which is running instead of the progress just silently
   * resetting partway through.
   */
  preparingLabel: string | null;
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
  preparingLabel: null,
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
      preparingLabel: null,
      logLines: [],
      error: null,
      exitCode: null,
    });

    let installBatchCount = 0;
    const unlistenInstall = await listenToInstallProgress((event) => {
      if (event.type === "started") installBatchCount += 1;
      set((state) => ({
        installProgress: applyInstallProgress(state.installProgress, event),
        preparingLabel:
          installBatchCount <= 1 ? "Downloading Minecraft…" : "Downloading a compatible Java…",
      }));
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
