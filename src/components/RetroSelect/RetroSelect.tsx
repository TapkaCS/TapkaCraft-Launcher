import type { ReactNode, SelectHTMLAttributes } from "react";
import { useId } from "react";

import styles from "./RetroSelect.module.css";

interface RetroSelectProps extends SelectHTMLAttributes<HTMLSelectElement> {
  label?: string;
  children: ReactNode;
}

export function RetroSelect({ label, className, id, children, ...rest }: RetroSelectProps) {
  const generatedId = useId();
  const selectId = id ?? generatedId;

  return (
    <div className={styles.wrapper}>
      {label ? (
        <label className={styles.label} htmlFor={selectId}>
          {label}
        </label>
      ) : null}
      <select
        id={selectId}
        className={[styles.select, className].filter(Boolean).join(" ")}
        {...rest}
      >
        {children}
      </select>
    </div>
  );
}
