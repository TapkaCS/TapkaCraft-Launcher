import { useEffect } from "react";

import { DashboardScreen } from "@/features/dashboard/DashboardScreen";
import { LoginScreen } from "@/features/login/LoginScreen";
import { useAuthStore } from "@/state/authStore";
import { useInstanceStore } from "@/state/instanceStore";
import { useSettingsStore } from "@/state/settingsStore";

/**
 * Root of the UI, switching between the login and dashboard screens purely
 * off `authStore`'s state machine - the same switch a real
 * `AccountService`-backed store will drive once Phase 4 replaces
 * `mockSignIn` with real Microsoft authentication. Also applies the
 * Appearance settings (theme/compact mode) to the document root, and loads
 * instances from disk once at startup - both need to happen above either
 * screen, not per-screen.
 */
export function AppShell() {
  const authStatus = useAuthStore((state) => state.state.status);
  const theme = useSettingsStore((state) => state.settings.appearance.theme);
  const compactMode = useSettingsStore((state) => state.settings.appearance.compactMode);
  const loadInstances = useInstanceStore((state) => state.loadInstances);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  useEffect(() => {
    document.documentElement.dataset.compact = String(compactMode);
  }, [compactMode]);

  // Instances are local files, independent of the Microsoft account state
  // machine, so this loads once regardless of auth status.
  useEffect(() => {
    void loadInstances();
  }, [loadInstances]);

  const isAuthenticated = authStatus === "authenticated" || authStatus === "refreshing";

  return isAuthenticated ? <DashboardScreen /> : <LoginScreen />;
}
