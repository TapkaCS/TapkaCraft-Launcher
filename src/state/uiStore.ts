import { create } from "zustand";

export type DashboardTab = "play" | "profiles" | "modrinth" | "modpacks" | "settings";

interface UiStore {
  activeTab: DashboardTab;
  setActiveTab: (tab: DashboardTab) => void;

  isNewProfileDialogOpen: boolean;
  openNewProfileDialog: () => void;
  closeNewProfileDialog: () => void;
}

export const useUiStore = create<UiStore>((set) => ({
  activeTab: "play",
  setActiveTab: (tab) => set({ activeTab: tab }),

  isNewProfileDialogOpen: false,
  openNewProfileDialog: () => set({ isNewProfileDialogOpen: true }),
  closeNewProfileDialog: () => set({ isNewProfileDialogOpen: false }),
}));
