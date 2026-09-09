/** Mirrors Rust's `LaunchEvent`/`OutputStream` (`src-tauri/src/core/launch.rs`), emitted as `launch://event`. */
export type LaunchEvent =
  | { type: "started"; pid: number }
  | { type: "output"; stream: "stdout" | "stderr"; line: string }
  | { type: "exited"; code: number | null };
