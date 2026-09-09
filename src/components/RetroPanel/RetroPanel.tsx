import type { HTMLAttributes, ReactNode } from "react";

import styles from "./RetroPanel.module.css";

interface RetroPanelProps extends HTMLAttributes<HTMLDivElement> {
  /** Heading content shown in the panel's header bar. Not the native `title` tooltip attribute. */
  heading?: ReactNode;
  headerExtra?: ReactNode;
  variant?: "raised" | "sunken" | "flat";
  noPadding?: boolean;
  children: ReactNode;
}

export function RetroPanel({
  heading,
  headerExtra,
  variant = "raised",
  noPadding = false,
  className,
  children,
  ...rest
}: RetroPanelProps) {
  const classes = [styles.panel, styles[variant], noPadding ? styles.noPadding : null, className]
    .filter(Boolean)
    .join(" ");

  return (
    <div className={classes} {...rest}>
      {heading ? (
        <div className={styles.header}>
          <span>{heading}</span>
          {headerExtra}
        </div>
      ) : null}
      <div className={styles.body}>{children}</div>
    </div>
  );
}
