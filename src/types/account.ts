/**
 * The account state machine described in the project spec's "ACCOUNT
 * MANAGEMENT" / "REMEMBER ME" sections: LoggedOut -> Authenticating ->
 * Authenticated, with Refreshing/AuthError reachable while trying to renew
 * an existing session.
 *
 * Modeled as a discriminated union (rather than a status enum plus a
 * separately-nullable account field) so illegal states - e.g. "authenticated
 * with no account data" - are unrepresentable and don't need runtime guards.
 *
 * Phase 1 only ever reaches these states via `authStore`'s mock sign-in -
 * there is no real Microsoft OAuth behind it yet (that's Phase 4). See
 * `src/lib/mock/mockData.ts`.
 */

export interface MinecraftAccount {
  /** Minecraft UUID (undashed or dashed - not yet meaningful for mock data). */
  id: string;
  username: string;
  /** URL/data-uri for a rendered skin head; absent falls back to a placeholder. */
  skinHeadUrl?: string;
  provider: "microsoft";
  /** ISO-8601 timestamp; used by the account switcher's "last used" hint. */
  lastUsed?: string;
}

export type AuthMachineState =
  | { status: "logged-out" }
  | { status: "authenticating" }
  | { status: "authenticated"; account: MinecraftAccount; rememberMe: boolean }
  | { status: "refreshing"; account: MinecraftAccount; rememberMe: boolean }
  | { status: "auth-error"; message: string; previousAccount?: MinecraftAccount };
