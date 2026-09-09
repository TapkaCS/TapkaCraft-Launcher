import { RetroCheckbox } from "@/components/RetroCheckbox/RetroCheckbox";
import { useSettingsStore } from "@/state/settingsStore";

import styles from "./SettingsSections.module.css";

export function AdvancedSection() {
  const advanced = useSettingsStore((state) => state.settings.advanced);
  const updateAdvanced = useSettingsStore((state) => state.updateAdvanced);

  return (
    <div className={styles.section}>
      <h2 className={styles.sectionTitle}>Advanced</h2>

      <div>
        <div className={styles.rowLabel}>
          <strong>Extra JVM arguments</strong>
          <span>
            Appended to the launch command built by LaunchEngine once it exists (Phase 3).
          </span>
        </div>
        <textarea
          className={styles.textArea}
          placeholder="-XX:+UseG1GC -XX:MaxGCPauseMillis=100"
          value={advanced.extraJvmArgs}
          onChange={(event) => updateAdvanced({ extraJvmArgs: event.target.value })}
        />
      </div>

      <div className={styles.checkboxRow}>
        <RetroCheckbox
          checked={advanced.debugLogging}
          onChange={(value) => updateAdvanced({ debugLogging: value })}
          label="Verbose debug logging"
          description="Widens what launcher.log captures once the logging pipeline exists. Never includes auth tokens or passwords, by design."
        />
      </div>
    </div>
  );
}
