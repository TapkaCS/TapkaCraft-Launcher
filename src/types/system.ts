/** Mirrors `MemorySuggestion`/`GpuInfo`/`GpuVendor` in `src-tauri/src/core/system.rs`. */

export interface MemorySuggestion {
  totalSystemMb: number;
  suggestedMinMb: number;
  suggestedMaxMb: number;
}

export type GpuVendor = "nvidia" | "amd" | "intel" | "apple" | "unknown";

export interface GpuInfo {
  name: string;
  vendor: GpuVendor;
}
