/**
 * The Settings structure from the project spec's "SETTINGS" section, kept
 * in localStorage (see `src/state/settingsStore.ts`) rather than a Rust-side
 * settings file for now. `downloads.concurrentDownloads` is genuinely read
 * by the Install button (Phase 3); `java.defaultMemoryMinMb/MaxMb` are
 * shown and can be filled from a real system-RAM suggestion, but are not
 * yet applied when a new profile is created (`NewInstanceInput` has no
 * memory field yet - new profiles still get `InstanceService`'s own
 * hardcoded default). Everything else here (resolution, custom Java path,
 * extra JVM args, appearance, ...) is still shape-only, read by nothing.
 */

export type ThemeMode = "retro" | "modern";

export interface GeneralSettings {
  closeLauncherOnGameStart: boolean;
  language: string;
}

export interface MinecraftSettings {
  defaultResolutionWidth: number;
  defaultResolutionHeight: number;
  useGlobalGameDirectory: boolean;
}

export interface JavaGlobalSettings {
  autoSelect: boolean;
  customPath: string | null;
  defaultMemoryMinMb: number;
  defaultMemoryMaxMb: number;
}

export interface DownloadSettings {
  concurrentDownloads: number;
}

export interface AppearanceSettings {
  theme: ThemeMode;
  compactMode: boolean;
}

export interface AdvancedSettings {
  extraJvmArgs: string;
  debugLogging: boolean;
}

export interface LauncherSettings {
  general: GeneralSettings;
  minecraft: MinecraftSettings;
  java: JavaGlobalSettings;
  downloads: DownloadSettings;
  appearance: AppearanceSettings;
  advanced: AdvancedSettings;
}

export const DEFAULT_SETTINGS: LauncherSettings = {
  general: {
    closeLauncherOnGameStart: false,
    language: "en",
  },
  minecraft: {
    defaultResolutionWidth: 1280,
    defaultResolutionHeight: 720,
    useGlobalGameDirectory: true,
  },
  java: {
    autoSelect: true,
    customPath: null,
    defaultMemoryMinMb: 1024,
    defaultMemoryMaxMb: 4096,
  },
  downloads: {
    concurrentDownloads: 4,
  },
  appearance: {
    theme: "retro",
    compactMode: false,
  },
  advanced: {
    extraJvmArgs: "",
    debugLogging: false,
  },
};
