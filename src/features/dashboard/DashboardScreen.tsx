import { AccountMenu } from "@/components/AccountMenu/AccountMenu";
import { NewProfileDialog } from "@/components/NewProfileDialog/NewProfileDialog";
import { PixelLogo } from "@/components/PixelLogo/PixelLogo";
import { RetroButton } from "@/components/RetroButton/RetroButton";
import { TopTabs } from "@/components/TopTabs/TopTabs";
import { SettingsScreen } from "@/features/settings/SettingsScreen";
import { useLauncherVersion } from "@/lib/useLauncherVersion";
import { useAuthStore } from "@/state/authStore";
import { useUiStore } from "@/state/uiStore";

import { ComingSoonPanel } from "./ComingSoonPanel";
import styles from "./DashboardScreen.module.css";
import { PlayTab } from "./PlayTab";
import { ProfilesTab } from "./ProfilesTab";

export function DashboardScreen() {
  const authState = useAuthStore((state) => state.state);
  const signOut = useAuthStore((state) => state.signOut);
  const activeTab = useUiStore((state) => state.activeTab);
  const setActiveTab = useUiStore((state) => state.setActiveTab);
  const isNewProfileDialogOpen = useUiStore((state) => state.isNewProfileDialogOpen);
  const closeNewProfileDialog = useUiStore((state) => state.closeNewProfileDialog);
  const version = useLauncherVersion();

  // AppShell only mounts this screen while authenticated/refreshing, but the
  // discriminated union still needs narrowing here for `authState.account`.
  if (authState.status !== "authenticated" && authState.status !== "refreshing") {
    return null;
  }

  return (
    <div className={styles.screen}>
      <header className={styles.header}>
        <PixelLogo size="sm" />
        <div className={styles.headerRight}>
          <AccountMenu account={authState.account} onSignOut={signOut} />
          <RetroButton variant="secondary" onClick={signOut}>
            Sign Out
          </RetroButton>
        </div>
      </header>

      <div className={styles.tabs}>
        <TopTabs active={activeTab} onChange={setActiveTab} />
      </div>

      <main className={styles.content}>
        {activeTab === "play" ? <PlayTab /> : null}
        {activeTab === "profiles" ? <ProfilesTab /> : null}
        {activeTab === "modrinth" ? (
          <ComingSoonPanel
            icon="compass"
            title="Modrinth"
            description="Search, browse and install mods, resource packs and shaders from Modrinth. This tab lands in Phase 6, once ModrinthService talks to the real API."
          />
        ) : null}
        {activeTab === "modpacks" ? (
          <ComingSoonPanel
            icon="box"
            title="Modpacks"
            description="Import and install .mrpack modpacks into their own isolated instance. This tab lands in Phase 7."
          />
        ) : null}
        {activeTab === "settings" ? <SettingsScreen /> : null}
      </main>

      <footer className={styles.footer}>
        <span>TapkaCraft Launcher{version ? ` v${version}` : ""}</span>
        <span>Playing together, since 2011. &#9829;</span>
      </footer>

      {isNewProfileDialogOpen ? <NewProfileDialog onClose={closeNewProfileDialog} /> : null}
    </div>
  );
}
