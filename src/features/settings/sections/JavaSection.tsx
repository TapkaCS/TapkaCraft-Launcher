import { useEffect, useState } from "react";

import { RetroCheckbox } from "@/components/RetroCheckbox/RetroCheckbox";
import { detectJavaRuntimes } from "@/lib/api/java";
import { getMemorySuggestion } from "@/lib/api/system";
import { isTauri } from "@/lib/tauri";
import { useSettingsStore } from "@/state/settingsStore";
import type { DetectedRuntime } from "@/types/java";
import type { MemorySuggestion } from "@/types/system";

import styles from "./SettingsSections.module.css";

function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export function JavaSection() {
  const java = useSettingsStore((state) => state.settings.java);
  const updateJava = useSettingsStore((state) => state.updateJava);

  const [runtimes, setRuntimes] = useState<DetectedRuntime[] | null>(null);
  const [runtimesLoading, setRuntimesLoading] = useState(isTauri);
  const [runtimesError, setRuntimesError] = useState<string | null>(null);

  const [suggestion, setSuggestion] = useState<MemorySuggestion | null>(null);
  const [suggestionError, setSuggestionError] = useState<string | null>(null);
  const [suggestionLoading, setSuggestionLoading] = useState(false);

  useEffect(() => {
    if (!isTauri) return;
    let cancelled = false;
    detectJavaRuntimes()
      .then((detected) => {
        if (!cancelled) setRuntimes(detected);
      })
      .catch((err) => {
        if (!cancelled) setRuntimesError(errorMessage(err));
      })
      .finally(() => {
        if (!cancelled) setRuntimesLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  async function handleSuggestMemory() {
    if (!isTauri) {
      setSuggestionError("Suggesting memory from system RAM requires the desktop app.");
      return;
    }
    setSuggestionLoading(true);
    setSuggestionError(null);
    try {
      const result = await getMemorySuggestion();
      setSuggestion(result);
      updateJava({
        defaultMemoryMinMb: result.suggestedMinMb,
        defaultMemoryMaxMb: result.suggestedMaxMb,
      });
    } catch (err) {
      setSuggestionError(errorMessage(err));
    } finally {
      setSuggestionLoading(false);
    }
  }

  return (
    <div className={styles.section}>
      <h2 className={styles.sectionTitle}>Java</h2>

      <div>
        <div className={styles.rowLabel}>
          <strong>Detected runtimes</strong>
        </div>
        {!isTauri ? (
          <div className={styles.emptyState}>
            Detecting installed Java runtimes requires the desktop app.
          </div>
        ) : runtimesLoading ? (
          <div className={styles.emptyState}>Detecting…</div>
        ) : runtimesError ? (
          <div className={styles.emptyState}>{runtimesError}</div>
        ) : runtimes && runtimes.length > 0 ? (
          <ul className={styles.runtimeList}>
            {runtimes.map((runtime) => (
              <li key={runtime.javaPath} className={styles.runtimeItem}>
                <span>
                  Java {runtime.majorVersion} ({runtime.versionString})
                  {runtime.vendor ? ` — ${runtime.vendor}` : ""}
                  {runtime.is64Bit ? "" : " · 32-bit"}
                </span>
                <span className={styles.runtimePath}>{runtime.javaPath}</span>
              </li>
            ))}
          </ul>
        ) : (
          <div className={styles.emptyState}>
            No Java runtimes detected on this machine (checked PATH, JAVA_HOME, and known install
            locations). Not a problem by itself - TapkaCraft downloads a matching Java automatically
            the first time a profile needs one it can't find.
          </div>
        )}
      </div>

      <div className={styles.checkboxRow}>
        <RetroCheckbox
          checked={java.autoSelect}
          onChange={(value) => updateJava({ autoSelect: value })}
          label="Automatically select a compatible Java runtime"
          description="Picks the closest detected runtime matching each version's required Java major version."
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

      <div>
        <button
          type="button"
          className={styles.suggestButton}
          onClick={() => void handleSuggestMemory()}
          disabled={suggestionLoading}
        >
          {suggestionLoading ? "Checking system RAM…" : "Suggest from system RAM"}
        </button>
        {suggestion ? (
          <p className={styles.hint}>
            This machine has {(suggestion.totalSystemMb / 1024).toFixed(1)} GB total RAM - suggested{" "}
            {suggestion.suggestedMinMb}&ndash;{suggestion.suggestedMaxMb} MB.
          </p>
        ) : null}
        {suggestionError ? <p className={styles.hint}>{suggestionError}</p> : null}
      </div>
    </div>
  );
}
