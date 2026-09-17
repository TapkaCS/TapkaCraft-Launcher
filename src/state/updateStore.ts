import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { create } from "zustand";

import { isTauri } from "@/lib/tauri";

type UpdateStatus =
  | "idle"
  | "checking"
  | "up-to-date"
  | "available"
  | "downloading"
  | "installing"
  | "check-error"
  | "install-error";

interface UpdateState {
  status: UpdateStatus;
  update: Update | null;
  error: string | null;
  /**
   * Checks the configured GitHub Releases endpoint
   * (`tauri.conf.json` > `plugins.updater.endpoints`) for a newer signed
   * build. Safe to call repeatedly - a check already running is not
   * restarted. A failure here lands in `check-error`, which `UpdateBanner`
   * deliberately stays silent about (this runs automatically on every
   * launch - offline or no release published yet shouldn't nag anyone) -
   * `GeneralSection`'s manual "Check for Updates" button reads the same
   * status for explicit feedback instead.
   */
  checkForUpdate: () => Promise<void>;
  /**
   * Downloads and installs the update found by `checkForUpdate`, then
   * restarts. A failure here lands in `install-error`, distinct from
   * `check-error` - this only ever runs after the user themselves clicked
   * "Restart & Update", so unlike a silent background check failing,
   * `UpdateBanner` does surface this one.
   */
  installUpdate: () => Promise<void>;
  /** Hides the update banner/error for the rest of this session. */
  dismiss: () => void;
}

function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export const useUpdateStore = create<UpdateState>((set, get) => ({
  status: "idle",
  update: null,
  error: null,

  checkForUpdate: async () => {
    if (!isTauri) return;
    const busy =
      get().status === "checking" ||
      get().status === "downloading" ||
      get().status === "installing";
    if (busy) return;

    set({ status: "checking", error: null });
    try {
      const update = await check();
      set(update ? { status: "available", update } : { status: "up-to-date", update: null });
    } catch (err) {
      set({ status: "check-error", error: errorMessage(err) });
    }
  },

  installUpdate: async () => {
    const update = get().update;
    if (!update) return;

    set({ status: "downloading", error: null });
    try {
      await update.downloadAndInstall();
      set({ status: "installing" });
      // No-op on Windows (downloadAndInstall already exits the app to launch
      // the installer there); required on macOS/Linux to actually restart.
      await relaunch();
    } catch (err) {
      set({ status: "install-error", error: errorMessage(err) });
    }
  },

  dismiss: () => set({ status: "idle", update: null, error: null }),
}));
