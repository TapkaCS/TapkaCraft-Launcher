import { RetroCheckbox } from "@/components/RetroCheckbox/RetroCheckbox";
import { RetroSelect } from "@/components/RetroSelect/RetroSelect";
import { useSettingsStore } from "@/state/settingsStore";
import type { ThemeMode } from "@/types/settings";

import styles from "./SettingsSections.module.css";

const THEMES: { id: ThemeMode; label: string }[] = [
  { id: "retro", label: "Retro" },
  { id: "modern", label: "Modern" },
];

export function AppearanceSection() {
  const appearance = useSettingsStore((state) => state.settings.appearance);
  const updateAppearance = useSettingsStore((state) => state.updateAppearance);
  const setTheme = useSettingsStore((state) => state.setTheme);

  return (
    <div className={styles.section}>
      <h2 className={styles.sectionTitle}>Appearance</h2>

      <div className={styles.row}>
        <div className={styles.rowLabel}>
          <strong>Theme</strong>
          <span>
            Retro: dirt textures, pixel type, beveled panels. Modern: flatter panels and cleaner
            type, same TapkaCraft identity. Same components either way - just different design
            tokens.
          </span>
        </div>
        <div className={styles.rowControl}>
          <RetroSelect
            aria-label="Theme"
            value={appearance.theme}
            onChange={(event) => setTheme(event.target.value as ThemeMode)}
          >
            {THEMES.map((theme) => (
              <option key={theme.id} value={theme.id}>
                {theme.label}
              </option>
            ))}
          </RetroSelect>
        </div>
      </div>

      <div className={styles.checkboxRow}>
        <RetroCheckbox
          checked={appearance.compactMode}
          onChange={(value) => updateAppearance({ compactMode: value })}
          label="Compact mode"
          description="Tightens spacing and text size throughout the launcher."
        />
      </div>
    </div>
  );
}
