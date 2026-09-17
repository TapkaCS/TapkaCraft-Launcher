import type { CSSProperties } from "react";

import { Icon } from "@/components/Icon/Icon";
import { LOADER_LABELS, type InstanceMeta, type LoaderKind } from "@/types/instance";

import styles from "./ProfileCard.module.css";

interface ProfileCardProps {
  instance: InstanceMeta;
  selected: boolean;
  onSelect: () => void;
  onToggleFavorite: () => void;
  onRename: () => void;
  onDelete: () => void;
}

const LOADER_ACCENTS: Record<LoaderKind, string> = {
  vanilla: "#7fd646",
  fabric: "#dbb98f",
  quilt: "#b79cf7",
  forge: "#e79a63",
  neoforge: "#e8b463",
};

function formatPlaytime(totalSeconds: number): string {
  if (totalSeconds <= 0) return "Not played yet";
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  if (hours > 0) return `${hours}h ${minutes}m played`;
  if (minutes > 0) return `${minutes}m played`;
  return "Played less than a minute";
}

function formatLastPlayed(lastPlayed: string | undefined): string | null {
  if (!lastPlayed) return null;
  const date = new Date(lastPlayed);
  if (Number.isNaN(date.getTime())) return null;
  return `Last played ${date.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" })}`;
}

/** The Profiles Library's card grid. `ProfileListItem` stays as-is for the Play tab's compact sidebar list - a different context, a different shape. */
export function ProfileCard({
  instance,
  selected,
  onSelect,
  onToggleFavorite,
  onRename,
  onDelete,
}: ProfileCardProps) {
  const accentStyle = { "--accent": LOADER_ACCENTS[instance.loader.type] } as CSSProperties;
  const subtitle =
    instance.loader.type === "vanilla"
      ? instance.minecraftVersion
      : `${instance.minecraftVersion} • ${LOADER_LABELS[instance.loader.type]}`;
  const lastPlayedLabel = formatLastPlayed(instance.lastPlayed);

  return (
    <div className={[styles.card, selected ? styles.selected : ""].filter(Boolean).join(" ")}>
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

      <button type="button" className={styles.main} onClick={onSelect}>
        <span className={styles.badge} style={accentStyle}>
          {instance.name.charAt(0).toUpperCase()}
        </span>
        <span className={styles.name}>{instance.name}</span>
        <span className={styles.subtitle}>{subtitle}</span>
        <span className={styles.stats}>
          {formatPlaytime(instance.playtimeSeconds)}
          {lastPlayedLabel ? ` · ${lastPlayedLabel}` : ""}
        </span>
      </button>

      <div className={styles.actions}>
        <button
          type="button"
          className={styles.iconButton}
          aria-label={`Rename ${instance.name}`}
          title="Rename"
          onClick={onRename}
        >
          <Icon name="pencil" size={14} />
        </button>
        <button
          type="button"
          className={[styles.iconButton, styles.iconButtonDanger].join(" ")}
          aria-label={`Delete ${instance.name}`}
          title="Delete"
          onClick={onDelete}
        >
          <Icon name="close" size={14} />
        </button>
      </div>
    </div>
  );
}
