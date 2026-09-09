/**
 * Typed wrappers around the account Tauri commands
 * (`src-tauri/src/commands/accounts.rs`). Always talk to the real Rust
 * backend - `authStore` decides whether to call these or reject with a
 * "requires the desktop app" message outside Tauri.
 */
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { MinecraftAccount, SignInEvent } from "@/types/account";

const SIGN_IN_PROGRESS_EVENT = "signin://progress";

export function beginSignIn(remember: boolean): Promise<MinecraftAccount> {
  return invoke<MinecraftAccount>("begin_sign_in", { remember });
}

export function tryRestoreSession(): Promise<MinecraftAccount | null> {
  return invoke<MinecraftAccount | null>("try_restore_session");
}

export function listAccounts(): Promise<MinecraftAccount[]> {
  return invoke<MinecraftAccount[]>("list_accounts");
}

export function getActiveAccount(): Promise<MinecraftAccount | null> {
  return invoke<MinecraftAccount | null>("active_account");
}

export function switchAccount(uuid: string): Promise<void> {
  return invoke<void>("switch_account", { uuid });
}

export function signOut(): Promise<void> {
  return invoke<void>("sign_out");
}

export function removeAccount(uuid: string): Promise<void> {
  return invoke<void>("remove_account", { uuid });
}

export function listenToSignInProgress(
  callback: (event: SignInEvent) => void,
): Promise<UnlistenFn> {
  return listen<SignInEvent>(SIGN_IN_PROGRESS_EVENT, (event) => callback(event.payload));
}
