import { create } from "zustand";

import { MOCK_ACCOUNT } from "@/lib/mock/mockData";
import type { AuthMachineState } from "@/types/account";

interface AuthStore {
  state: AuthMachineState;
  /** Bound to the "Remember me" checkbox on the login screen. */
  rememberMePreference: boolean;
  setRememberMePreference: (value: boolean) => void;

  /**
   * Drives LoggedOut -> Authenticating -> Authenticated using
   * `MOCK_ACCOUNT`. This is a stand-in for the real Microsoft sign-in flow
   * (loopback authorization-code + PKCE -> Xbox Live -> XSTS -> Minecraft
   * Services), which is Phase 4 and requires an Azure AD app registration
   * the project owner has not supplied yet. It exists so the login/
   * dashboard routing and the state machine itself can be built and tested
   * now, ahead of the real backend.
   */
  mockSignIn: () => Promise<void>;
  signOut: () => void;
}

const MOCK_SIGN_IN_DELAY_MS = 900;

function delay(ms: number) {
  return new Promise<void>((resolve) => setTimeout(resolve, ms));
}

export const useAuthStore = create<AuthStore>((set, get) => ({
  state: { status: "logged-out" },
  rememberMePreference: true,

  setRememberMePreference: (value) => set({ rememberMePreference: value }),

  mockSignIn: async () => {
    if (get().state.status === "authenticating") return;
    set({ state: { status: "authenticating" } });
    await delay(MOCK_SIGN_IN_DELAY_MS);
    set({
      state: {
        status: "authenticated",
        account: MOCK_ACCOUNT,
        rememberMe: get().rememberMePreference,
      },
    });
  },

  signOut: () => set({ state: { status: "logged-out" } }),
}));
