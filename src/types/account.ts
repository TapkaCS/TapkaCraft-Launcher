/**
 * The account state machine described in the project spec's "ACCOUNT
 * MANAGEMENT" section: LoggedOut -> Authenticating -> Authenticated, with
 * Refreshing/AuthError reachable while trying to renew an existing
 * session. Modeled as a discriminated union (rather than a status enum
 * plus a separately-nullable account field) so illegal states - e.g.
 * "authenticated with no account data" - are unrepresentable and don't
 * need runtime guards.
 *
 * `MinecraftAccount` mirrors Rust's `StoredAccount`
 * (`src-tauri/src/core/accounts/model.rs`) field-for-field - this app only
 * ever signs in through Microsoft, so there's no separate provider tag to
 * carry.
 */

export interface MinecraftAccount {
  minecraftUuid: string;
  username: string;
  skinUrl?: string;
  /** ISO-8601 timestamp; used by the account switcher's "last used" hint. */
  lastUsed: string;
}

export type AuthMachineState =
  | { status: "logged-out" }
  | { status: "authenticating" }
  | { status: "authenticated"; account: MinecraftAccount }
  | { status: "refreshing"; account: MinecraftAccount }
  | { status: "auth-error"; message: string; previousAccount?: MinecraftAccount };

/** Mirrors Rust's `SignInEvent` (`src-tauri/src/core/accounts/service.rs`), emitted as `signin://progress`. */
export type SignInEvent =
  | { type: "openingBrowser" }
  | { type: "waitingForBrowser" }
  | { type: "exchangingCode" }
  | { type: "authenticatingWithXbox" }
  | { type: "authenticatingWithXsts" }
  | { type: "authenticatingWithMinecraft" }
  | { type: "checkingOwnership" }
  | { type: "fetchingProfile" };

export const SIGN_IN_STEP_LABELS: Record<SignInEvent["type"], string> = {
  openingBrowser: "Opening your browser…",
  waitingForBrowser: "Waiting for you to sign in…",
  exchangingCode: "Confirming with Microsoft…",
  authenticatingWithXbox: "Signing in to Xbox Live…",
  authenticatingWithXsts: "Verifying your Xbox profile…",
  authenticatingWithMinecraft: "Signing in to Minecraft…",
  checkingOwnership: "Checking your Minecraft license…",
  fetchingProfile: "Loading your profile…",
};
