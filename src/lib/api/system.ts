import { invoke } from "@tauri-apps/api/core";

import type { GpuInfo, MemorySuggestion } from "@/types/system";

export function getMemorySuggestion(): Promise<MemorySuggestion> {
  return invoke<MemorySuggestion>("get_memory_suggestion");
}

export function getGpuInfo(): Promise<GpuInfo[]> {
  return invoke<GpuInfo[]>("get_gpu_info");
}
