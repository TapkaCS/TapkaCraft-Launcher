import { Icon } from "@/components/Icon/Icon";
import { LauncherDialog } from "@/components/LauncherDialog/LauncherDialog";
import { PixelLogo } from "@/components/PixelLogo/PixelLogo";
import { RetroButton } from "@/components/RetroButton/RetroButton";
import { RetroCheckbox } from "@/components/RetroCheckbox/RetroCheckbox";
import { useLauncherVersion } from "@/lib/useLauncherVersion";
import { useAuthStore } from "@/state/authStore";
import { SIGN_IN_STEP_LABELS } from "@/types/account";

import styles from "./LoginScreen.module.css";

export function LoginScreen() {
  const authState = useAuthStore((state) => state.state);
  const signInStep = useAuthStore((state) => state.signInStep);
  const rememberMe = useAuthStore((state) => state.rememberMePreference);
  const setRememberMe = useAuthStore((state) => state.setRememberMePreference);
  const signIn = useAuthStore((state) => state.signIn);
  const continueAsGuest = useAuthStore((state) => state.continueAsGuest);
  const version = useLauncherVersion();
  const isAuthenticating = authState.status === "authenticating";
  const statusLabel = signInStep ? SIGN_IN_STEP_LABELS[signInStep] : "Signing in…";

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
          onClick={() => void signIn()}
          disabled={isAuthenticating}
        >
          {isAuthenticating ? statusLabel : "Sign in with Microsoft"}
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
          onClick={() => continueAsGuest()}
          disabled={isAuthenticating}
        >
          Continue without an account
        </button>
        <p className={styles.guestHint}>
          Browse profiles and Modrinth without signing in. Playing still needs a Microsoft account.
        </p>

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
