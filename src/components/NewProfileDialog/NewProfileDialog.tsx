import { useEffect, useState, type FormEvent, type MouseEvent } from "react";

import { LauncherDialog } from "@/components/LauncherDialog/LauncherDialog";
import { RetroButton } from "@/components/RetroButton/RetroButton";
import { RetroSelect } from "@/components/RetroSelect/RetroSelect";
import { getVersionManifest } from "@/lib/api/versions";
import { isTauri } from "@/lib/tauri";
import { useInstanceStore } from "@/state/instanceStore";
import type { LoaderKind } from "@/types/instance";

import styles from "./NewProfileDialog.module.css";

/**
 * Used until the real Mojang manifest loads (or outside Tauri, where it
 * never will) - a fixed, possibly-stale list beats an empty picker.
 */
const FALLBACK_MINECRAFT_VERSIONS = [
  "1.21.1",
  "1.20.4",
  "1.20.1",
  "1.19.4",
  "1.18.2",
  "1.16.5",
  "1.8.9",
];

const LOADERS: { value: LoaderKind; label: string }[] = [
  { value: "vanilla", label: "Vanilla" },
  { value: "fabric", label: "Fabric" },
  { value: "quilt", label: "Quilt" },
  { value: "forge", label: "Forge" },
  { value: "neoforge", label: "NeoForge" },
];

interface NewProfileDialogProps {
  onClose: () => void;
}

export function NewProfileDialog({ onClose }: NewProfileDialogProps) {
  const createInstance = useInstanceStore((state) => state.createInstance);
  const [name, setName] = useState("");
  const [versions, setVersions] = useState<string[]>(FALLBACK_MINECRAFT_VERSIONS);
  const [versionsLoading, setVersionsLoading] = useState(isTauri);
  const [minecraftVersion, setMinecraftVersion] = useState(FALLBACK_MINECRAFT_VERSIONS[0]);
  const [loader, setLoader] = useState<LoaderKind>("vanilla");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const trimmedName = name.trim();
  const canSubmit = trimmedName.length > 0 && !isSubmitting;

  useEffect(() => {
    if (!isTauri) return;
    let cancelled = false;
    getVersionManifest()
      .then((summaries) => {
        if (cancelled || summaries.length === 0) return;
        const ids = summaries.map((summary) => summary.id);
        setVersions(ids);
        setMinecraftVersion(ids[0]);
      })
      .catch(() => {
        // Real Mojang manifest unreachable (offline, no local cache yet
        // either) - keep the fallback list rather than blocking the dialog.
      })
      .finally(() => {
        if (!cancelled) setVersionsLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (!canSubmit) return;
    setIsSubmitting(true);
    setError(null);
    try {
      await createInstance({ name: trimmedName, minecraftVersion, loader });
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
      <LauncherDialog heading="New Profile" className={styles.dialog}>
        <form className={styles.form} onSubmit={(event) => void handleSubmit(event)}>
          <label className={styles.field}>
            <span className={styles.fieldLabel}>Profile name</span>
            <input
              className={styles.input}
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder="e.g. Performance"
              autoFocus
              disabled={isSubmitting}
            />
          </label>

          <RetroSelect
            label="Minecraft version"
            value={minecraftVersion}
            onChange={(event) => setMinecraftVersion(event.target.value)}
            disabled={isSubmitting || versionsLoading}
          >
            {versions.map((version) => (
              <option key={version} value={version}>
                {version}
              </option>
            ))}
          </RetroSelect>

          <RetroSelect
            label="Loader"
            value={loader}
            onChange={(event) => setLoader(event.target.value as LoaderKind)}
            disabled={isSubmitting}
          >
            {LOADERS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </RetroSelect>

          {loader !== "vanilla" ? (
            <p className={styles.note}>
              Installing {LOADERS.find((option) => option.value === loader)?.label} isn&apos;t
              supported yet - only Vanilla instances can be installed so far.
            </p>
          ) : null}

          {error ? <p className={styles.error}>{error}</p> : null}

          <p className={styles.note}>
            {isTauri
              ? "Creates the instance folder now (mods/saves/resourcepacks/shaderpacks/screenshots). Use the Install button on the Play tab afterward to download the Minecraft files themselves."
              : "Browser preview: this profile is kept in memory only for this session, not written to disk."}
          </p>

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
              {isSubmitting ? "Creating…" : "Create"}
            </RetroButton>
          </div>
        </form>
      </LauncherDialog>
    </div>
  );
}
