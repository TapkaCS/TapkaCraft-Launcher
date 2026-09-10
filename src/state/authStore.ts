import { create } from "zustand";

import {
  beginSignIn,
  listenToSignInProgress,
  signOut as signOutCommand,
  tryRestoreSession,
} from "@/lib/api/accounts";
import { isTauri } from "@/lib/tauri";
import type { AuthMachineState, SignInEvent } from "@/types/account";

interface AuthStore {
  state: AuthMachineState;
  /** Bound to the "Remember me" checkbox on the login screen. */
  rememberMePreference: boolean;
  setRememberMePreference: (value: boolean) => void;
  /** The sign-in flow's current step, for a live status line - null when not signing in. */
  signInStep: SignInEvent["type"] | null;

  /**
   * Tries to silently resume a previous sign-in (a stored refresh token
   * for whichever account was last active) without opening a browser.
   * Called once at startup; leaves the state at `logged-out` on any
   * failure - reason doesn't matter here, the login screen is the answer
   * either way.
   */
  restoreSession: () => Promise<void>;
  /** The real loopback Microsoft sign-in flow - opens the system browser. */
  signIn: () => Promise<void>;
  signOut: () => Promise<void>;
  /**
   * Skips straight to the dashboard without a Microsoft account. Purely a
   * local state change - no backend call, since there's nothing to
   * authenticate. `signOut` (which also covers leaving guest mode) returns
   * here to `logged-out`.
   */
  continueAsGuest: () => void;
}

function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export const useAuthStore = create<AuthStore>((set, get) => ({
  state: { status: "logged-out" },
  rememberMePreference: true,
  signInStep: null,

  setRememberMePreference: (value) => set({ rememberMePreference: value }),

  restoreSession: async () => {
    if (!isTauri) return;
    try {
      const account = await tryRestoreSession();
      if (account) {
        set({ state: { status: "authenticated", account } });
      }
    } catch {
      // Stay logged out - the login screen is the right fallback for any
      // failure here (no stored account, expired token, offline, ...).
    }
  },

  signIn: async () => {
    if (get().state.status === "authenticating") return;
    set({ state: { status: "authenticating" }, signInStep: null });

    if (!isTauri) {
      set({
        state: {
          status: "auth-error",
          message: "Signing in requires the desktop app, not the browser preview.",
        },
        signInStep: null,
      });
      return;
    }

    const unlisten = await listenToSignInProgress((event) => set({ signInStep: event.type }));

    try {
      const account = await beginSignIn(get().rememberMePreference);
      set({ state: { status: "authenticated", account }, signInStep: null });
    } catch (err) {
      set({ state: { status: "auth-error", message: errorMessage(err) }, signInStep: null });
    } finally {
      unlisten();
    }
  },

  signOut: async () => {
    if (isTauri && get().state.status !== "guest") {
      try {
        await signOutCommand();
      } catch {
        // Sign-out proceeds locally either way - there's nothing the UI
        // can usefully do about a failure to clear server-side state here.
      }
    }
    set({ state: { status: "logged-out" }, signInStep: null });
  },

  continueAsGuest: () => set({ state: { status: "guest" }, signInStep: null }),
}));
