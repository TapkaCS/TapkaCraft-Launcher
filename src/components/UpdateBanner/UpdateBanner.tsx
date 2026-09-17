import { RetroButton } from "@/components/RetroButton/RetroButton";
import { useUpdateStore } from "@/state/updateStore";

import styles from "./UpdateBanner.module.css";

/**
 * Rendered above either the login or dashboard screen (see `AppShell`), so
 * it's visible regardless of sign-in state. Deliberately silent for
 * "checking"/"up-to-date"/"check-error" - a background check failing
 * (offline, no release published yet) runs on every launch and isn't worth
 * interrupting anyone over; the same store's status is shown, with error
 * text, in Settings > General for the manual "Check for Updates" button
 * instead. "install-error" is different: it only happens after the user
 * themselves clicked "Restart & Update" right here, so silently making the
 * banner vanish would leave them with no idea anything went wrong.
 */
export function UpdateBanner() {
  const status = useUpdateStore((state) => state.status);
  const update = useUpdateStore((state) => state.update);
  const error = useUpdateStore((state) => state.error);
  const installUpdate = useUpdateStore((state) => state.installUpdate);
  const dismiss = useUpdateStore((state) => state.dismiss);

  if (status === "available" && update) {
    return (
      <div className={styles.banner}>
        <span>
          TapkaCraft Launcher {update.version} is available (you have {update.currentVersion}).
        </span>
        <div className={styles.actions}>
          <RetroButton variant="primary" onClick={() => void installUpdate()}>
            Restart &amp; Update
          </RetroButton>
          <RetroButton variant="secondary" onClick={dismiss}>
            Later
          </RetroButton>
        </div>
      </div>
    );
  }

  if (status === "downloading") {
    return (
      <div className={styles.banner}>
        <span>Downloading update…</span>
      </div>
    );
  }

  if (status === "installing") {
    return (
      <div className={styles.banner}>
        <span>Installing update - restarting…</span>
      </div>
    );
  }

  if (status === "install-error") {
    return (
      <div className={styles.banner} data-variant="error">
        <span>
          Update failed: {error ?? "unknown error"}. You can download the installer manually from
          the GitHub releases page instead.
        </span>
        <div className={styles.actions}>
          <RetroButton variant="primary" onClick={() => void installUpdate()}>
            Try Again
          </RetroButton>
          <RetroButton variant="secondary" onClick={dismiss}>
            Dismiss
          </RetroButton>
        </div>
      </div>
    );
  }

  return null;
}
