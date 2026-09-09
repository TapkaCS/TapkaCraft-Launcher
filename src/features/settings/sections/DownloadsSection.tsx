import { useSettingsStore } from "@/state/settingsStore";

import styles from "./SettingsSections.module.css";

export function DownloadsSection() {
  const downloads = useSettingsStore((state) => state.settings.downloads);
  const updateDownloads = useSettingsStore((state) => state.updateDownloads);

  return (
    <div className={styles.section}>
      <h2 className={styles.sectionTitle}>Downloads</h2>

      <div className={styles.row}>
        <div className={styles.rowLabel}>
          <strong>Concurrent downloads</strong>
          <span>How many files DownloadManager fetches in parallel once it exists (Phase 3).</span>
        </div>
        <div className={styles.rowControl}>
          <input
            type="number"
            className={styles.numberInput}
            aria-label="Concurrent downloads"
            min={1}
            max={16}
            value={downloads.concurrentDownloads}
            onChange={(event) =>
              updateDownloads({
                concurrentDownloads: Math.min(16, Math.max(1, Number(event.target.value) || 1)),
              })
            }
          />
        </div>
      </div>
    </div>
  );
}
