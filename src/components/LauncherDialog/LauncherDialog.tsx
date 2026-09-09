import type { HTMLAttributes, ReactNode } from "react";

import styles from "./LauncherDialog.module.css";

interface LauncherDialogProps extends HTMLAttributes<HTMLDivElement> {
  /** Heading content shown above the dialog body. Not the native `title` tooltip attribute. */
  heading?: ReactNode;
  children: ReactNode;
}

export function LauncherDialog({ heading, className, children, ...rest }: LauncherDialogProps) {
  return (
    <div className={[styles.dialog, className].filter(Boolean).join(" ")} {...rest}>
      {heading ? <p className={styles.title}>{heading}</p> : null}
      {children}
    </div>
  );
}
