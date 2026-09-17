import { create } from "zustand";

import { listenToLanEvents, startLanDiscovery, stopLanDiscovery } from "@/lib/api/lan";
import { isTauri } from "@/lib/tauri";
import type { LanGame } from "@/types/lan";

type LanStatus = "idle" | "listening" | "error";

interface LanState {
  games: LanGame[];
  status: LanStatus;
  error: string | null;
  /** Starts the background multicast listener. Safe to call repeatedly - a no-op once already listening. */
  start: () => Promise<void>;
  stop: () => Promise<void>;
}

function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

// Module-level rather than in the store's state: an `UnlistenFn` isn't data
// a component should render off of, just a handle `stop` needs to clean up.
let unlisten: (() => void) | null = null;

export const useLanStore = create<LanState>((set, get) => ({
  games: [],
  status: "idle",
  error: null,

  start: async () => {
    if (get().status === "listening") return;
    if (!isTauri) {
      set({
        status: "error",
        error: "LAN Discovery requires the desktop app, not the browser preview.",
      });
      return;
    }

    set({ error: null });
    try {
      // Registered before `startLanDiscovery` resolves so no broadcast
      // received right after the backend starts listening can be missed.
      unlisten = await listenToLanEvents((event) => {
        set({ games: event.games });
      });
      await startLanDiscovery();
      set({ status: "listening" });
    } catch (err) {
      unlisten?.();
      unlisten = null;
      set({ status: "error", error: errorMessage(err) });
    }
  },

  stop: async () => {
    unlisten?.();
    unlisten = null;
    set({ status: "idle", games: [] });
    if (!isTauri) return;
    try {
      await stopLanDiscovery();
    } catch {
      // Best-effort - the tab is closing either way, nothing left to show
      // an error for.
    }
  },
}));
