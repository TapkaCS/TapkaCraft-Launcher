import markUrl from "@/assets/brand/mark.png";

import styles from "./PixelLogo.module.css";

interface PixelLogoProps {
  size?: "sm" | "md" | "lg";
  showWordmark?: boolean;
  className?: string;
}

export function PixelLogo({ size = "md", showWordmark = true, className }: PixelLogoProps) {
  return (
    <div className={[styles.logo, styles[size], className].filter(Boolean).join(" ")}>
      <img src={markUrl} alt="TapkaCraft Launcher" className={styles.mark} />
      {showWordmark ? (
        <span className={styles.wordmark} aria-hidden="true">
          <span className={styles.primary}>TAPKACRAFT</span>
          <span className={styles.secondary}>LAUNCHER</span>
        </span>
      ) : null}
    </div>
  );
}
