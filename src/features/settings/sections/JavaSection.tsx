import { RetroCheckbox } from "@/components/RetroCheckbox/RetroCheckbox";
import { useSettingsStore } from "@/state/settingsStore";

import styles from "./SettingsSections.module.css";

export function JavaSection() {
  const java = useSettingsStore((state) => state.settings.java);
  const updateJava = useSettingsStore((state) => state.updateJava);

  return (
    <div className={styles.section}>
      <h2 className={styles.sectionTitle}>Java</h2>

      <div>
        <div className={styles.rowLabel}>
          <strong>Detected runtimes</strong>
        </div>
        <div className={styles.emptyState}>
          No Java runtimes detected yet. Automatic detection (registry + PATH + JAVA_HOME on
          Windows) is implemented in Phase 3 by JavaManager.
        </div>
      </div>

      <div className={styles.checkboxRow}>
        <RetroCheckbox
          checked={java.autoSelect}
          onChange={(value) => updateJava({ autoSelect: value })}
          label="Automatically select a compatible Java runtime"
          description="Picks the right Java major version for each Minecraft version once JavaManager exists."
        />
      </div>

      <div className={styles.row}>
        <div className={styles.rowLabel}>
          <strong>Custom Java path</strong>
          <span>
            Overrides auto-selection. Point this at a javaw.exe if you need a specific runtime.
          </span>
        </div>
        <div className={styles.rowControl}>
          <input
            type="text"
            className={styles.textInput}
            placeholder="Not set"
            disabled={java.autoSelect}
            value={java.customPath ?? ""}
            onChange={(event) => updateJava({ customPath: event.target.value || null })}
          />
        </div>
      </div>

      <div className={styles.row}>
        <div className={styles.rowLabel}>
          <strong>Default memory allocation</strong>
          <span>Applied to new profiles; each profile can still override it.</span>
        </div>
        <div className={styles.rowControl}>
          <div className={styles.pair}>
            <input
              type="number"
              className={styles.numberInput}
              aria-label="Minimum memory (MB)"
              min={512}
              step={256}
              value={java.defaultMemoryMinMb}
              onChange={(event) =>
                updateJava({ defaultMemoryMinMb: Number(event.target.value) || 0 })
              }
            />
            <span>&ndash;</span>
            <input
              type="number"
              className={styles.numberInput}
              aria-label="Maximum memory (MB)"
              min={512}
              step={256}
              value={java.defaultMemoryMaxMb}
              onChange={(event) =>
                updateJava({ defaultMemoryMaxMb: Number(event.target.value) || 0 })
              }
            />
            <span>MB</span>
          </div>
        </div>
      </div>
    </div>
  );
}
