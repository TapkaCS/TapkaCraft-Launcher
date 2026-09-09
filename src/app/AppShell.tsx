import { useEffect } from "react";

import { DashboardScreen } from "@/features/dashboard/DashboardScreen";
import { LoginScreen } from "@/features/login/LoginScreen";
import { useAuthStore } from "@/state/authStore";
import { useSettingsStore } from "@/state/settingsStore";

/**
 * Root of the UI, switching between the login and dashboard screens purely
 * off `authStore`'s state machine - the same switch a real
 * `AccountService`-backed store will drive once Phase 4 replaces
 * `mockSignIn` with real Microsoft authentication. Also applies the
 * Appearance settings (theme/compact mode) to the document root, since
 * that has to happen above both screens.
 */
export function AppShell() {
  const authStatus = useAuthStore((state) => state.state.status);
  const theme = useSettingsStore((state) => state.settings.appearance.theme);
  const compactMode = useSettingsStore((state) => state.settings.appearance.compactMode);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  useEffect(() => {
    document.documentElement.dataset.compact = String(compactMode);
  }, [compactMode]);

  const isAuthenticated = authStatus === "authenticated" || authStatus === "refreshing";

  return isAuthenticated ? <DashboardScreen /> : <LoginScreen />;
}
