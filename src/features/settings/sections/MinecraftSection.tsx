import { RetroCheckbox } from "@/components/RetroCheckbox/RetroCheckbox";
import { useSettingsStore } from "@/state/settingsStore";

import styles from "./SettingsSections.module.css";

export function MinecraftSection() {
  const minecraft = useSettingsStore((state) => state.settings.minecraft);
  const updateMinecraft = useSettingsStore((state) => state.updateMinecraft);

  return (
    <div className={styles.section}>
      <h2 className={styles.sectionTitle}>Minecraft</h2>

      <div className={styles.row}>
        <div className={styles.rowLabel}>
          <strong>Default window resolution</strong>
          <span>Used when launching a profile that doesn&apos;t override its own resolution.</span>
        </div>
        <div className={styles.rowControl}>
          <div className={styles.pair}>
            <input
              type="number"
              className={styles.numberInput}
              aria-label="Default width"
              min={640}
              value={minecraft.defaultResolutionWidth}
              onChange={(event) =>
                updateMinecraft({ defaultResolutionWidth: Number(event.target.value) || 0 })
              }
            />
            <span>&times;</span>
            <input
              type="number"
              className={styles.numberInput}
              aria-label="Default height"
              min={480}
              value={minecraft.defaultResolutionHeight}
              onChange={(event) =>
                updateMinecraft({ defaultResolutionHeight: Number(event.target.value) || 0 })
              }
            />
          </div>
        </div>
      </div>

      <div className={styles.checkboxRow}>
        <RetroCheckbox
          checked={minecraft.useGlobalGameDirectory}
          onChange={(value) => updateMinecraft({ useGlobalGameDirectory: value })}
          label="Use a shared game directory behavior for all instances"
          description="When off, each instance can be pointed at its own separate game directory."
        />
      </div>
    </div>
  );
}
