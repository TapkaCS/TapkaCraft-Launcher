/**
 * The Settings structure from the project spec's "SETTINGS" section.
 * Phase 1 only implements the shape and in-memory/localStorage state - none
 * of these values are read by a real backend service yet (no Java
 * detection, no download manager, no launch engine), so changing them here
 * currently has no effect beyond the Settings screen itself. See
 * `src/state/settingsStore.ts`.
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
