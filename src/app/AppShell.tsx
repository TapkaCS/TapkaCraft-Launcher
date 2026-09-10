import { useEffect } from "react";

import { DashboardScreen } from "@/features/dashboard/DashboardScreen";
import { LoginScreen } from "@/features/login/LoginScreen";
import { useAuthStore } from "@/state/authStore";
import { useInstanceStore } from "@/state/instanceStore";
import { useSettingsStore } from "@/state/settingsStore";

/**
 * Root of the UI, switching between the login and dashboard screens purely
 * off `authStore`'s state machine, which real Microsoft authentication
 * (Phase 4) drives via a loopback OAuth flow. Also applies the Appearance
 * settings (theme/compact mode) to the document root, loads instances from
 * disk once at startup, and tries a silent session restore so a
 * previously-remembered account skips straight to the dashboard instead of
 * showing the login screen again - all need to happen above either screen,
 * not per-screen.
 */
export function AppShell() {
  const authStatus = useAuthStore((state) => state.state.status);
  const restoreSession = useAuthStore((state) => state.restoreSession);
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

  useEffect(() => {
    void restoreSession();
    // Runs once at startup only - re-running on every store update would
    // re-attempt the silent refresh after every sign-out too.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const showDashboard =
    authStatus === "authenticated" || authStatus === "refreshing" || authStatus === "guest";

  return showDashboard ? <DashboardScreen /> : <LoginScreen />;
}
