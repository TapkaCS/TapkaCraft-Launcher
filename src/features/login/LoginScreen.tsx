import { Icon } from "@/components/Icon/Icon";
import { LauncherDialog } from "@/components/LauncherDialog/LauncherDialog";
import { PixelLogo } from "@/components/PixelLogo/PixelLogo";
import { RetroButton } from "@/components/RetroButton/RetroButton";
import { RetroCheckbox } from "@/components/RetroCheckbox/RetroCheckbox";
import { useLauncherVersion } from "@/lib/useLauncherVersion";
import { useAuthStore } from "@/state/authStore";

import styles from "./LoginScreen.module.css";

export function LoginScreen() {
  const authState = useAuthStore((state) => state.state);
  const rememberMe = useAuthStore((state) => state.rememberMePreference);
  const setRememberMe = useAuthStore((state) => state.setRememberMePreference);
  const mockSignIn = useAuthStore((state) => state.mockSignIn);
  const version = useLauncherVersion();
  const isAuthenticating = authState.status === "authenticating";

  return (
    <div className={`${styles.screen} tc-dirt-bg`}>
      <PixelLogo size="lg" className={styles.logo} />

      <LauncherDialog>
        <p className={styles.tagline}>Play TapkaCraft with your Microsoft account</p>

        {authState.status === "auth-error" ? (
          <p className={styles.errorBanner}>{authState.message}</p>
        ) : null}

        <RetroButton
          variant="microsoft"
          size="lg"
          fullWidth
          icon={<Icon name="microsoft" size={20} />}
          onClick={() => void mockSignIn()}
          disabled={isAuthenticating}
        >
          {isAuthenticating ? "Signing in…" : "Sign in with Microsoft"}
        </RetroButton>

        <div className={styles.rememberRow}>
          <RetroCheckbox
            checked={rememberMe}
            onChange={setRememberMe}
            label="Remember me"
            description={"Stay signed in so you don't have to\nlog in every time you play."}
          />
        </div>

        <button
          type="button"
          className={styles.helpLink}
          disabled
          title="TapkaCraft account support isn't set up yet"
        >
          Need help with your account?
        </button>
      </LauncherDialog>

      {version ? <span className={styles.version}>v{version}</span> : null}
    </div>
  );
}
