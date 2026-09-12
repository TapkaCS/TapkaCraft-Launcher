import { RetroButton } from "@/components/RetroButton/RetroButton";
import { useUpdateStore } from "@/state/updateStore";

import styles from "./UpdateBanner.module.css";

/**
 * Rendered above either the login or dashboard screen (see `AppShell`), so
 * it's visible regardless of sign-in state. Deliberately silent for
 * "checking"/"up-to-date"/"error" - a background check failing (offline, no
 * release published yet) isn't worth interrupting anyone over. The same
 * store's status is also shown, with error text, in Settings > General for
 * the manual "Check for Updates" button.
 */
export function UpdateBanner() {
  const status = useUpdateStore((state) => state.status);
  const update = useUpdateStore((state) => state.update);
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

  return null;
}
