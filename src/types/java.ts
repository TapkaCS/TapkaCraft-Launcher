/** Mirrors `DetectedRuntime` in `src-tauri/src/core/java/runtime.rs`. */
export interface DetectedRuntime {
  javaPath: string;
  majorVersion: number;
  versionString: string;
  is64Bit: boolean;
  vendor: string | null;
}
