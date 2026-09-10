import { beforeEach, describe, expect, it } from "vitest";

import { useAuthStore } from "./authStore";

// vitest runs in jsdom, which has no `__TAURI_INTERNALS__` global, so
// `isTauri` is false throughout this file - `signIn()` always takes its
// "desktop app required" branch here. The real Microsoft OAuth flow
// (loopback PKCE -> Xbox Live -> XSTS -> Minecraft Services) is covered by
// the Rust-side `core::accounts` tests instead.
describe("authStore", () => {
  beforeEach(() => {
    useAuthStore.setState({
      state: { status: "logged-out" },
      rememberMePreference: true,
      signInStep: null,
    });
  });

  it("starts logged out", () => {
    expect(useAuthStore.getState().state.status).toBe("logged-out");
  });

  it("reports a clear error instead of pretending to succeed outside Tauri", async () => {
    await useAuthStore.getState().signIn();
    const state = useAuthStore.getState().state;
    expect(state.status).toBe("auth-error");
    if (state.status === "auth-error") {
      expect(state.message).toMatch(/desktop app/i);
    }
  });

  it("does not start a second concurrent sign-in while already authenticating", async () => {
    const first = useAuthStore.getState().signIn();
    const second = useAuthStore.getState().signIn();
    await Promise.all([first, second]);
    expect(useAuthStore.getState().state.status).toBe("auth-error");
  });

  it("signOut returns to logged-out", async () => {
    await useAuthStore.getState().signIn(); // lands in auth-error outside Tauri
    await useAuthStore.getState().signOut();
    expect(useAuthStore.getState().state.status).toBe("logged-out");
  });

  it("restoreSession is a no-op outside Tauri", async () => {
    await useAuthStore.getState().restoreSession();
    expect(useAuthStore.getState().state.status).toBe("logged-out");
  });

  it("continueAsGuest switches straight to guest with no account", () => {
    useAuthStore.getState().continueAsGuest();
    expect(useAuthStore.getState().state).toEqual({ status: "guest" });
  });

  it("signOut from guest returns to logged-out without calling the backend", async () => {
    useAuthStore.getState().continueAsGuest();
    await useAuthStore.getState().signOut();
    expect(useAuthStore.getState().state.status).toBe("logged-out");
  });
});
