import { RetroButton } from "@/components/RetroButton/RetroButton";
import { RetroCheckbox } from "@/components/RetroCheckbox/RetroCheckbox";
import { RetroSelect } from "@/components/RetroSelect/RetroSelect";
import { useLauncherVersion } from "@/lib/useLauncherVersion";
import { useSettingsStore } from "@/state/settingsStore";
import { useUpdateStore } from "@/state/updateStore";

import styles from "./SettingsSections.module.css";

const STATUS_LABEL: Record<string, string> = {
  checking: "Checking…",
  "up-to-date": "You're up to date.",
  downloading: "Downloading update…",
  installing: "Installing update - restarting…",
};

export function GeneralSection() {
  const general = useSettingsStore((state) => state.settings.general);
  const updateGeneral = useSettingsStore((state) => state.updateGeneral);
  const version = useLauncherVersion();
  const updateStatus = useUpdateStore((state) => state.status);
  const update = useUpdateStore((state) => state.update);
  const updateError = useUpdateStore((state) => state.error);
  const checkForUpdate = useUpdateStore((state) => state.checkForUpdate);
  const installUpdate = useUpdateStore((state) => state.installUpdate);

  const isBusy =
    updateStatus === "checking" || updateStatus === "downloading" || updateStatus === "installing";

  return (
    <div className={styles.section}>
      <h2 className={styles.sectionTitle}>General</h2>

      <div className={styles.row}>
        <div className={styles.rowLabel}>
          <strong>Updates</strong>
          <span>
            {version ? `You have version ${version}. ` : ""}
            Checks the latest release published on GitHub.
          </span>
          {updateStatus === "error" && updateError ? (
            <span className={styles.hint}>Check failed: {updateError}</span>
          ) : STATUS_LABEL[updateStatus] ? (
            <span className={styles.hint}>{STATUS_LABEL[updateStatus]}</span>
          ) : null}
        </div>
        <div className={styles.rowControl}>
          {updateStatus === "available" && update ? (
            <RetroButton variant="primary" onClick={() => void installUpdate()}>
              Restart &amp; Update to {update.version}
            </RetroButton>
          ) : (
            <RetroButton
              variant="secondary"
              disabled={isBusy}
              onClick={() => void checkForUpdate()}
            >
              {updateStatus === "checking" ? "Checking…" : "Check for Updates"}
            </RetroButton>
          )}
        </div>
      </div>

      <div className={styles.checkboxRow}>
        <RetroCheckbox
          checked={general.closeLauncherOnGameStart}
          onChange={(value) => updateGeneral({ closeLauncherOnGameStart: value })}
          label="Close the launcher when the game starts"
          description="Leave unchecked to keep TapkaCraft Launcher open while you play."
        />
      </div>

      <div className={styles.row}>
        <div className={styles.rowLabel}>
          <strong>Language</strong>
          <span>
            Interface translations aren&apos;t implemented yet - this saves your preference for
            later.
          </span>
        </div>
        <div className={styles.rowControl}>
          <RetroSelect
            aria-label="Language"
            value={general.language}
            onChange={(event) => updateGeneral({ language: event.target.value })}
          >
            <option value="en">English</option>
            <option value="cs">Čeština</option>
          </RetroSelect>
        </div>
      </div>
    </div>
  );
}
