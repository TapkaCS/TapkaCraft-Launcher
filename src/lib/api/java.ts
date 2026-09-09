import { invoke } from "@tauri-apps/api/core";

import type { DetectedRuntime } from "@/types/java";

export function detectJavaRuntimes(): Promise<DetectedRuntime[]> {
  return invoke<DetectedRuntime[]>("detect_java_runtimes");
}
