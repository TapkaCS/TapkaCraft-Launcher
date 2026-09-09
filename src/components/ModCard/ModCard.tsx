import type { CSSProperties } from "react";

import type { FeaturedModCard } from "@/lib/mock/mockData";

import styles from "./ModCard.module.css";

function formatCount(value: number): string {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}k`;
  return String(value);
}

export function ModCard({ mod }: { mod: FeaturedModCard }) {
  const accentStyle = { "--accent": mod.accent } as CSSProperties;

  return (
    <div className={styles.card}>
      <div className={styles.badge} style={accentStyle}>
        {mod.badge}
      </div>
      <div className={styles.info}>
        <p className={styles.title}>{mod.title}</p>
        <p className={styles.author}>By {mod.author}</p>
        <p className={styles.description}>{mod.description}</p>
        <p className={styles.stats}>
          <span>&#8595; {formatCount(mod.downloads)}</span>
          <span>&#9829; {formatCount(mod.follows)}</span>
        </p>
      </div>
    </div>
  );
}
