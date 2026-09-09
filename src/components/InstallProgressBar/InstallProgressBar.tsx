import type { InstallProgress } from "@/state/installStore";

import styles from "./InstallProgressBar.module.css";

interface InstallProgressBarProps {
  progress: InstallProgress;
  label: string;
}

/** Shared by every download-backed progress display (instance install, launch's pre-launch install, Modrinth mod install) - one `install://progress` event shape, one look. */
export function InstallProgressBar({ progress, label }: InstallProgressBarProps) {
  return (
    <div className={styles.wrap}>
      <span className={styles.label}>
        {progress.totalFiles > 0
          ? `${label} ${progress.completedFiles}/${progress.totalFiles} files`
          : `${label}…`}
        {progress.currentLabel ? ` — ${progress.currentLabel}` : ""}
      </span>
      <div className={styles.track}>
        <div
          className={styles.fill}
          style={{
            width:
              progress.totalFiles > 0
                ? `${Math.min(100, (progress.completedFiles / progress.totalFiles) * 100)}%`
                : "6%",
          }}
        />
      </div>
    </div>
  );
}
