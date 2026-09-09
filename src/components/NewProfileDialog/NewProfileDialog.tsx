import { useState, type FormEvent, type MouseEvent } from "react";

import { LauncherDialog } from "@/components/LauncherDialog/LauncherDialog";
import { RetroButton } from "@/components/RetroButton/RetroButton";
import { RetroSelect } from "@/components/RetroSelect/RetroSelect";
import { useInstanceStore } from "@/state/instanceStore";
import type { LoaderKind } from "@/types/instance";

import styles from "./NewProfileDialog.module.css";

const MINECRAFT_VERSIONS = ["1.21.1", "1.20.4", "1.20.1", "1.19.4", "1.18.2", "1.16.5", "1.8.9"];

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
  const [minecraftVersion, setMinecraftVersion] = useState(MINECRAFT_VERSIONS[0]);
  const [loader, setLoader] = useState<LoaderKind>("vanilla");

  const trimmedName = name.trim();
  const canSubmit = trimmedName.length > 0;

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (!canSubmit) return;
    createInstance({ name: trimmedName, minecraftVersion, loader });
    onClose();
  }

  function handleOverlayClick(event: MouseEvent<HTMLDivElement>) {
    if (event.target === event.currentTarget) onClose();
  }

  return (
    <div className={styles.overlay} role="presentation" onMouseDown={handleOverlayClick}>
      <LauncherDialog heading="New Profile" className={styles.dialog}>
        <form className={styles.form} onSubmit={handleSubmit}>
          <label className={styles.field}>
            <span className={styles.fieldLabel}>Profile name</span>
            <input
              className={styles.input}
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder="e.g. Performance"
              autoFocus
            />
          </label>

          <RetroSelect
            label="Minecraft version"
            value={minecraftVersion}
            onChange={(event) => setMinecraftVersion(event.target.value)}
          >
            {MINECRAFT_VERSIONS.map((version) => (
              <option key={version} value={version}>
                {version}
              </option>
            ))}
          </RetroSelect>

          <RetroSelect
            label="Loader"
            value={loader}
            onChange={(event) => setLoader(event.target.value as LoaderKind)}
          >
            {LOADERS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </RetroSelect>

          <p className={styles.note}>
            Profiles created here are kept for this session only - real instance folders and
            downloads arrive in a later update.
          </p>

          <div className={styles.actions}>
            <RetroButton type="button" variant="secondary" onClick={onClose}>
              Cancel
            </RetroButton>
            <RetroButton type="submit" variant="primary" disabled={!canSubmit}>
              Create
            </RetroButton>
          </div>
        </form>
      </LauncherDialog>
    </div>
  );
}
