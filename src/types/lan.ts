/**
 * Mirrors Rust's `LanGame`/`LanEvent` (`src-tauri/src/core/lan.rs`), emitted
 * as `lan://event` while LAN Discovery is listening.
 */

export interface LanGame {
  host: string;
  port: number;
  motd: string;
}

export type LanEvent = { type: "snapshot"; games: LanGame[] };
