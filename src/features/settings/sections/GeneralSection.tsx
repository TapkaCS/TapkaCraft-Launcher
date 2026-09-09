import { RetroCheckbox } from "@/components/RetroCheckbox/RetroCheckbox";
import { RetroSelect } from "@/components/RetroSelect/RetroSelect";
import { useSettingsStore } from "@/state/settingsStore";

import styles from "./SettingsSections.module.css";

export function GeneralSection() {
  const general = useSettingsStore((state) => state.settings.general);
  const updateGeneral = useSettingsStore((state) => state.updateGeneral);

  return (
    <div className={styles.section}>
      <h2 className={styles.sectionTitle}>General</h2>

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
