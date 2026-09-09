import { useEffect, useState } from "react";

import { RetroCheckbox } from "@/components/RetroCheckbox/RetroCheckbox";
import { getGpuInfo } from "@/lib/api/system";
import { isTauri } from "@/lib/tauri";
import { useSettingsStore } from "@/state/settingsStore";
import type { GpuInfo } from "@/types/system";

import styles from "./SettingsSections.module.css";

export function AdvancedSection() {
  const advanced = useSettingsStore((state) => state.settings.advanced);
  const updateAdvanced = useSettingsStore((state) => state.updateAdvanced);

  const [gpus, setGpus] = useState<GpuInfo[] | null>(null);
  const [gpuLoading, setGpuLoading] = useState(isTauri);

  useEffect(() => {
    if (!isTauri) return;
    let cancelled = false;
    getGpuInfo()
      .then((detected) => {
        if (!cancelled) setGpus(detected);
      })
      .catch(() => {
        if (!cancelled) setGpus([]);
      })
      .finally(() => {
        if (!cancelled) setGpuLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <div className={styles.section}>
      <h2 className={styles.sectionTitle}>Advanced</h2>

      <div>
        <div className={styles.rowLabel}>
          <strong>Detected graphics hardware</strong>
          <span>Diagnostic only - best-effort, and nothing here changes launch behavior yet.</span>
        </div>
        <div className={styles.emptyState}>
          {!isTauri
            ? "GPU detection requires the desktop app."
            : gpuLoading
              ? "Detecting…"
              : gpus && gpus.length > 0
                ? gpus.map((gpu) => gpu.name).join(", ")
                : "No GPU detected via this platform's standard tooling."}
        </div>
      </div>

      <div>
        <div className={styles.rowLabel}>
          <strong>Extra JVM arguments</strong>
          <span>
            Appended to the launch command once launching is wired up - LaunchEngine itself is
            ready, but is gated on Microsoft sign-in, which isn&apos;t built yet.
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
