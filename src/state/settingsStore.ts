import { create } from "zustand";
import { persist } from "zustand/middleware";

import {
  DEFAULT_SETTINGS,
  type AdvancedSettings,
  type AppearanceSettings,
  type DownloadSettings,
  type GeneralSettings,
  type JavaGlobalSettings,
  type LauncherSettings,
  type MinecraftSettings,
  type ThemeMode,
} from "@/types/settings";

interface SettingsStore {
  settings: LauncherSettings;
  updateGeneral: (patch: Partial<GeneralSettings>) => void;
  updateMinecraft: (patch: Partial<MinecraftSettings>) => void;
  updateJava: (patch: Partial<JavaGlobalSettings>) => void;
  updateDownloads: (patch: Partial<DownloadSettings>) => void;
  updateAppearance: (patch: Partial<AppearanceSettings>) => void;
  updateAdvanced: (patch: Partial<AdvancedSettings>) => void;
  setTheme: (theme: ThemeMode) => void;
}

/**
 * Settings persist to the webview's localStorage for now, which is real and
 * works, but is a stand-in for the eventual Rust-backed settings file under
 * the app's roaming data directory (`core::paths::AppPaths::settings_dir`).
 * Swapping the storage backend later does not require changing this
 * store's public API.
 */
export const useSettingsStore = create<SettingsStore>()(
  persist(
    (set, get) => ({
      settings: DEFAULT_SETTINGS,

      updateGeneral: (patch) =>
        set({ settings: { ...get().settings, general: { ...get().settings.general, ...patch } } }),
      updateMinecraft: (patch) =>
        set({
          settings: { ...get().settings, minecraft: { ...get().settings.minecraft, ...patch } },
        }),
      updateJava: (patch) =>
        set({ settings: { ...get().settings, java: { ...get().settings.java, ...patch } } }),
      updateDownloads: (patch) =>
        set({
          settings: { ...get().settings, downloads: { ...get().settings.downloads, ...patch } },
        }),
      updateAppearance: (patch) =>
        set({
          settings: {
            ...get().settings,
            appearance: { ...get().settings.appearance, ...patch },
          },
        }),
      updateAdvanced: (patch) =>
        set({
          settings: { ...get().settings, advanced: { ...get().settings.advanced, ...patch } },
        }),

      setTheme: (theme) =>
        set({
          settings: { ...get().settings, appearance: { ...get().settings.appearance, theme } },
        }),
    }),
    { name: "tapkacraft-settings" },
  ),
);
