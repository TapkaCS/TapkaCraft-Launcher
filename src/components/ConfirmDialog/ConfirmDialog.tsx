import type { MouseEvent, ReactNode } from "react";

import { LauncherDialog } from "@/components/LauncherDialog/LauncherDialog";
import { RetroButton } from "@/components/RetroButton/RetroButton";

import styles from "./ConfirmDialog.module.css";

interface ConfirmDialogProps {
  heading: string;
  message: ReactNode;
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
  isBusy?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

/** Generic confirm/cancel dialog - used for destructive actions like Delete Profile. */
export function ConfirmDialog({
  heading,
  message,
  confirmLabel = "Confirm",
  cancelLabel = "Cancel",
  danger = false,
  isBusy = false,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  function handleOverlayClick(event: MouseEvent<HTMLDivElement>) {
    if (!isBusy && event.target === event.currentTarget) onCancel();
  }

  return (
    <div className={styles.overlay} role="presentation" onMouseDown={handleOverlayClick}>
      <LauncherDialog heading={heading} className={styles.dialog}>
        <p className={styles.message}>{message}</p>
        <div className={styles.actions}>
          <RetroButton type="button" variant="secondary" onClick={onCancel} disabled={isBusy}>
            {cancelLabel}
          </RetroButton>
          <RetroButton
            type="button"
            variant={danger ? "danger" : "primary"}
            onClick={onConfirm}
            disabled={isBusy}
          >
            {isBusy ? "Working…" : confirmLabel}
          </RetroButton>
        </div>
      </LauncherDialog>
    </div>
  );
}
