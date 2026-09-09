import { useState, type FormEvent, type MouseEvent } from "react";

import { LauncherDialog } from "@/components/LauncherDialog/LauncherDialog";
import { RetroButton } from "@/components/RetroButton/RetroButton";
import { useInstanceStore } from "@/state/instanceStore";
import type { InstanceMeta } from "@/types/instance";

import styles from "./RenameProfileDialog.module.css";

interface RenameProfileDialogProps {
  instance: InstanceMeta;
  onClose: () => void;
}

export function RenameProfileDialog({ instance, onClose }: RenameProfileDialogProps) {
  const renameInstance = useInstanceStore((state) => state.renameInstance);
  const [name, setName] = useState(instance.name);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const trimmed = name.trim();
  const canSubmit = trimmed.length > 0 && !isSubmitting;

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (!canSubmit) return;
    if (trimmed === instance.name) {
      onClose();
      return;
    }
    setIsSubmitting(true);
    setError(null);
    try {
      await renameInstance(instance.id, trimmed);
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setIsSubmitting(false);
    }
  }

  function handleOverlayClick(event: MouseEvent<HTMLDivElement>) {
    if (!isSubmitting && event.target === event.currentTarget) onClose();
  }

  return (
    <div className={styles.overlay} role="presentation" onMouseDown={handleOverlayClick}>
      <LauncherDialog heading="Rename Profile" className={styles.dialog}>
        <form className={styles.form} onSubmit={(event) => void handleSubmit(event)}>
          <input
            className={styles.input}
            value={name}
            onChange={(event) => setName(event.target.value)}
            autoFocus
            disabled={isSubmitting}
          />

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
