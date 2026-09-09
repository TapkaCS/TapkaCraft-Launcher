import type { ButtonHTMLAttributes, ReactNode } from "react";

import styles from "./RetroButton.module.css";

type Variant = "primary" | "secondary" | "danger" | "ghost" | "microsoft";
type Size = "md" | "lg";

interface RetroButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
  icon?: ReactNode;
  fullWidth?: boolean;
}

export function RetroButton({
  variant = "secondary",
  size = "md",
  icon,
  fullWidth = false,
  className,
  children,
  ...rest
}: RetroButtonProps) {
  const classes = [
    styles.button,
    styles[variant],
    styles[size],
    fullWidth ? styles.fullWidth : null,
    className,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <button className={classes} {...rest}>
      {icon ? <span className={styles.icon}>{icon}</span> : null}
      <span>{children}</span>
    </button>
  );
}
