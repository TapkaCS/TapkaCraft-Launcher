import { Icon } from "@/components/Icon/Icon";
import { InstallProgressBar } from "@/components/InstallProgressBar/InstallProgressBar";
import { RetroPanel } from "@/components/RetroPanel/RetroPanel";
import { useInstallStore, type InstallProgress } from "@/state/installStore";
import { useInstanceStore } from "@/state/instanceStore";
import { useLaunchStore } from "@/state/launchStore";
import { useModpackStore } from "@/state/modpackStore";
import { useModrinthStore } from "@/state/modrinthStore";
import type { SearchHit } from "@/types/modrinth";

import styles from "./DownloadsTab.module.css";

interface DownloadEntry {
  key: string;
  label: string;
  progress: InstallProgress;
}

function hitTitle(results: SearchHit[], id: string | null, fallback: string): string {
  if (!id) return fallback;
  return results.find((hit) => hit.project_id === id)?.title ?? fallback;
}

/**
 * A real aggregation of every download-backed operation currently running
 * elsewhere in the app - instance installs, the pre-launch install Play
 * triggers, Modrinth mod installs, modpack installs/imports - read straight
 * from those stores' own state, not a separate queue this tab pretends to
 * manage. Nothing is listed until real progress data exists for it, so
 * there's never a fake zeroed-out progress bar here.
 */
export function DownloadsTab() {
  const instances = useInstanceStore((state) => state.instances);

  function instanceName(id: string | null): string {
    if (!id) return "a profile";
    return instances.find((instance) => instance.id === id)?.name ?? id;
  }

  const installingInstanceId = useInstallStore((state) => state.installingInstanceId);
  const installProgress = useInstallStore((state) => state.progress);

  const launchingInstanceId = useLaunchStore((state) => state.launchingInstanceId);
  const launchPhase = useLaunchStore((state) => state.phase);
  const launchInstallProgress = useLaunchStore((state) => state.installProgress);
  const preparingLabel = useLaunchStore((state) => state.preparingLabel);

  const modInstallingProjectId = useModrinthStore((state) => state.installingProjectId);
  const modInstallProgress = useModrinthStore((state) => state.installProgress);
  const modResults = useModrinthStore((state) => state.results);

  const modpackInstallingProjectId = useModpackStore((state) => state.installingProjectId);
  const modpackIsImportingFile = useModpackStore((state) => state.isImportingFile);
  const modpackInstallProgress = useModpackStore((state) => state.installProgress);
  const modpackResults = useModpackStore((state) => state.results);

  const entries: DownloadEntry[] = [];

  if (installingInstanceId && installProgress) {
    entries.push({
      key: "install",
      label: `Installing "${instanceName(installingInstanceId)}"`,
      progress: installProgress,
    });
  }
  if (launchingInstanceId && launchPhase === "preparing" && launchInstallProgress) {
    const verb = (preparingLabel ?? "Preparing").replace(/…+$/u, "");
    entries.push({
      key: "launch",
      label: `${verb} — "${instanceName(launchingInstanceId)}"`,
      progress: launchInstallProgress,
    });
  }
  if (modInstallingProjectId && modInstallProgress) {
    entries.push({
      key: "mod",
      label: `Installing mod "${hitTitle(modResults, modInstallingProjectId, "a mod")}"`,
      progress: modInstallProgress,
    });
  }
  if (modpackInstallingProjectId && modpackInstallProgress) {
    entries.push({
      key: "modpack",
      label: `Installing modpack "${hitTitle(modpackResults, modpackInstallingProjectId, "a modpack")}"`,
      progress: modpackInstallProgress,
    });
  }
  if (modpackIsImportingFile && modpackInstallProgress) {
    entries.push({
      key: "modpack-import",
      label: "Importing modpack file",
      progress: modpackInstallProgress,
    });
  }

  return (
    <div className={styles.layout}>
      <RetroPanel heading="Downloads">
        {entries.length === 0 ? (
          <div className={styles.empty}>
            <div className={styles.emptyIcon}>
              <Icon name="download" size={28} />
            </div>
            <p className={styles.emptyTitle}>Nothing downloading right now</p>
            <p className={styles.emptyDescription}>
              Installs you start from Play, Modrinth or Modpacks show up here while they run.
            </p>
          </div>
        ) : (
          <ul className={styles.entryList}>
            {entries.map((entry) => (
              <li key={entry.key} className={styles.entry}>
                <InstallProgressBar progress={entry.progress} label={entry.label} />
              </li>
            ))}
          </ul>
        )}
      </RetroPanel>
    </div>
  );
}
