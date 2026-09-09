import type { CSSProperties } from "react";

import { Icon } from "@/components/Icon/Icon";
import { LOADER_LABELS, type InstanceMeta, type LoaderKind } from "@/types/instance";

import styles from "./ProfileListItem.module.css";

interface ProfileListItemProps {
  instance: InstanceMeta;
  selected: boolean;
  onSelect: () => void;
  onToggleFavorite: () => void;
}

const LOADER_ACCENTS: Record<LoaderKind, string> = {
  vanilla: "#7fd646",
  fabric: "#dbb98f",
  quilt: "#b79cf7",
  forge: "#e79a63",
  neoforge: "#e8b463",
};

export function ProfileListItem({
  instance,
  selected,
  onSelect,
  onToggleFavorite,
}: ProfileListItemProps) {
  const accentStyle = { "--accent": LOADER_ACCENTS[instance.loader.type] } as CSSProperties;
  const subtitle =
    instance.loader.type === "vanilla"
      ? instance.minecraftVersion
      : `${instance.minecraftVersion} • ${LOADER_LABELS[instance.loader.type]}`;

  return (
    <div className={[styles.row, selected ? styles.selected : ""].filter(Boolean).join(" ")}>
      <button type="button" className={styles.main} onClick={onSelect}>
        <span className={styles.badge} style={accentStyle}>
          {instance.name.charAt(0).toUpperCase()}
        </span>
        <span className={styles.text}>
          <span className={styles.name}>{instance.name}</span>
          <span className={styles.subtitle}>{subtitle}</span>
        </span>
      </button>
      <button
        type="button"
        className={styles.favorite}
        aria-pressed={instance.favorite}
        aria-label={
          instance.favorite
            ? `Remove ${instance.name} from favorites`
            : `Add ${instance.name} to favorites`
        }
        onClick={onToggleFavorite}
      >
        <Icon name={instance.favorite ? "star" : "starOutline"} size={16} />
      </button>
    </div>
  );
}
