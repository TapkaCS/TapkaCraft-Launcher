import { useState, type FormEvent, type MouseEvent } from "react";

import { LauncherDialog } from "@/components/LauncherDialog/LauncherDialog";
import { RetroButton } from "@/components/RetroButton/RetroButton";
import { getMemorySuggestion } from "@/lib/api/system";
import { isTauri } from "@/lib/tauri";
import { useInstanceStore } from "@/state/instanceStore";
import { LOADER_LABELS, type InstanceMeta } from "@/types/instance";

import styles from "./ProfileEditorDialog.module.css";

interface ProfileEditorDialogProps {
  instance: InstanceMeta;
  onClose: () => void;
}

function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

/**
 * Edits a profile's own name and Java memory allocation. Minecraft
 * version/loader aren't editable here - changing either on an existing
 * instance is Smart Upgrade territory (`core::instances::model::InstanceUpdate`'s
 * own doc comment), not a plain metadata edit.
 */
export function ProfileEditorDialog({ instance, onClose }: ProfileEditorDialogProps) {
  const renameInstance = useInstanceStore((state) => state.renameInstance);
  const updateJavaSettings = useInstanceStore((state) => state.updateJavaSettings);

  const [name, setName] = useState(instance.name);
  const [memoryMinMb, setMemoryMinMb] = useState(instance.java.memoryMinMb);
  const [memoryMaxMb, setMemoryMaxMb] = useState(instance.java.memoryMaxMb);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [suggestionLoading, setSuggestionLoading] = useState(false);
  const [suggestionError, setSuggestionError] = useState<string | null>(null);

  const trimmedName = name.trim();
  const memoryValid = memoryMinMb > 0 && memoryMaxMb >= memoryMinMb;
  const canSubmit = trimmedName.length > 0 && memoryValid && !isSubmitting;

  const versionSubtitle =
    instance.loader.type === "vanilla"
      ? instance.minecraftVersion
      : `${instance.minecraftVersion} • ${LOADER_LABELS[instance.loader.type]}`;

  async function handleSuggestMemory() {
    if (!isTauri) {
      setSuggestionError("Suggesting memory from system RAM requires the desktop app.");
      return;
    }
    setSuggestionLoading(true);
    setSuggestionError(null);
    try {
      const result = await getMemorySuggestion();
      setMemoryMinMb(result.suggestedMinMb);
      setMemoryMaxMb(result.suggestedMaxMb);
    } catch (err) {
      setSuggestionError(errorMessage(err));
    } finally {
      setSuggestionLoading(false);
    }
  }

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (!canSubmit) return;
    setIsSubmitting(true);
    setError(null);
    try {
      if (trimmedName !== instance.name) {
        await renameInstance(instance.id, trimmedName);
      }
      if (memoryMinMb !== instance.java.memoryMinMb || memoryMaxMb !== instance.java.memoryMaxMb) {
        await updateJavaSettings(instance.id, { memoryMinMb, memoryMaxMb });
      }
      onClose();
    } catch (err) {
      setError(errorMessage(err));
      setIsSubmitting(false);
    }
  }

  function handleOverlayClick(event: MouseEvent<HTMLDivElement>) {
    if (!isSubmitting && event.target === event.currentTarget) onClose();
  }

  return (
    <div className={styles.overlay} role="presentation" onMouseDown={handleOverlayClick}>
      <LauncherDialog heading="Edit Profile" className={styles.dialog}>
        <form className={styles.form} onSubmit={(event) => void handleSubmit(event)}>
          <label className={styles.field}>
            <span className={styles.fieldLabel}>Profile name</span>
            <input
              className={styles.input}
              value={name}
              onChange={(event) => setName(event.target.value)}
              autoFocus
              disabled={isSubmitting}
            />
          </label>

          <div className={styles.field}>
            <span className={styles.fieldLabel}>Minecraft version</span>
            <p className={styles.readOnlyValue}>{versionSubtitle}</p>
            <span className={styles.hint}>
              Changing version or loader isn&apos;t supported yet - create a new profile instead.
            </span>
          </div>

          <div className={styles.field}>
            <span className={styles.fieldLabel}>Memory allocation</span>
            <div className={styles.memoryRow}>
              <input
                type="number"
                className={styles.numberInput}
                aria-label="Minimum memory (MB)"
                min={512}
                step={256}
                value={memoryMinMb}
                onChange={(event) => setMemoryMinMb(Number(event.target.value) || 0)}
                disabled={isSubmitting}
              />
              <span>&ndash;</span>
              <input
                type="number"
                className={styles.numberInput}
                aria-label="Maximum memory (MB)"
                min={512}
                step={256}
                value={memoryMaxMb}
                onChange={(event) => setMemoryMaxMb(Number(event.target.value) || 0)}
                disabled={isSubmitting}
              />
              <span>MB</span>
              <button
                type="button"
                className={styles.suggestButton}
                onClick={() => void handleSuggestMemory()}
                disabled={isSubmitting || suggestionLoading}
              >
                {suggestionLoading ? "Checking…" : "Suggest"}
              </button>
            </div>
            {!memoryValid ? (
              <span className={styles.hint}>Maximum must be at least the minimum.</span>
            ) : null}
            {suggestionError ? <span className={styles.hint}>{suggestionError}</span> : null}
          </div>

          {error ? <p className={styles.error}>{error}</p> : null}

          <div className={styles.actions}>
            <RetroButton
              type="button"
              variant="secondary"
              onClick={onClose}
              disabled={isSubmitting}
            >
              Cancel
            </RetroButton>
            <RetroButton type="submit" variant="primary" disabled={!canSubmit}>
              {isSubmitting ? "Saving…" : "Save"}
            </RetroButton>
          </div>
        </form>
      </LauncherDialog>
    </div>
  );
}
