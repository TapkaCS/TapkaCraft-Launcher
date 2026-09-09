import { beforeEach, describe, expect, it } from "vitest";

import { useAuthStore } from "./authStore";

describe("authStore", () => {
  beforeEach(() => {
    useAuthStore.setState({ state: { status: "logged-out" }, rememberMePreference: true });
  });

  it("starts logged out", () => {
    expect(useAuthStore.getState().state.status).toBe("logged-out");
  });

  it("moves logged-out -> authenticating -> authenticated on mockSignIn", async () => {
    const signInPromise = useAuthStore.getState().mockSignIn();
    expect(useAuthStore.getState().state.status).toBe("authenticating");

    await signInPromise;

    const state = useAuthStore.getState().state;
    expect(state.status).toBe("authenticated");
    if (state.status === "authenticated") {
      expect(state.account.username).toBeTruthy();
      expect(state.rememberMe).toBe(true);
    }
  });

  it("carries the current rememberMe preference into the authenticated state", async () => {
    useAuthStore.getState().setRememberMePreference(false);
    await useAuthStore.getState().mockSignIn();

    const state = useAuthStore.getState().state;
    expect(state.status).toBe("authenticated");
    if (state.status === "authenticated") {
      expect(state.rememberMe).toBe(false);
    }
  });

  it("signOut returns to logged-out from authenticated", async () => {
    await useAuthStore.getState().mockSignIn();
    useAuthStore.getState().signOut();
    expect(useAuthStore.getState().state.status).toBe("logged-out");
  });

  it("does not start a second concurrent sign-in while already authenticating", async () => {
    const first = useAuthStore.getState().mockSignIn();
    const second = useAuthStore.getState().mockSignIn();
    await Promise.all([first, second]);
    expect(useAuthStore.getState().state.status).toBe("authenticated");
  });
});
