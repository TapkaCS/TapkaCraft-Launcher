import { useState } from "react";

import { ConfirmDialog } from "@/components/ConfirmDialog/ConfirmDialog";
import { Icon } from "@/components/Icon/Icon";
import { ProfileListItem } from "@/components/ProfileListItem/ProfileListItem";
import { RenameProfileDialog } from "@/components/RenameProfileDialog/RenameProfileDialog";
import { RetroButton } from "@/components/RetroButton/RetroButton";
import { useInstanceStore } from "@/state/instanceStore";
import { useUiStore } from "@/state/uiStore";
import type { InstanceMeta } from "@/types/instance";

import styles from "./ProfilesTab.module.css";

export function ProfilesTab() {
  const instances = useInstanceStore((state) => state.instances);
  const status = useInstanceStore((state) => state.status);
  const error = useInstanceStore((state) => state.error);
  const selectedId = useInstanceStore((state) => state.selectedInstanceId);
  const selectInstance = useInstanceStore((state) => state.selectInstance);
  const toggleFavorite = useInstanceStore((state) => state.toggleFavorite);
  const deleteInstance = useInstanceStore((state) => state.deleteInstance);
  const loadInstances = useInstanceStore((state) => state.loadInstances);
  const openNewProfileDialog = useUiStore((state) => state.openNewProfileDialog);

  const [renaming, setRenaming] = useState<InstanceMeta | null>(null);
  const [deleting, setDeleting] = useState<InstanceMeta | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);

  async function handleConfirmDelete() {
    if (!deleting) return;
    setIsDeleting(true);
    setDeleteError(null);
    try {
      await deleteInstance(deleting.id);
      setDeleting(null);
    } catch (err) {
      setDeleteError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsDeleting(false);
    }
  }

  if (status === "loading" || status === "idle") {
    return (
      <div className={styles.wrapper}>
        <div className={styles.state}>Loading profiles…</div>
      </div>
    );
  }

  if (status === "error") {
    return (
      <div className={styles.wrapper}>
        <div className={[styles.state, styles.stateError].join(" ")}>
          <p>Couldn&apos;t load profiles: {error}</p>
          <RetroButton variant="secondary" onClick={() => void loadInstances()}>
            Retry
          </RetroButton>
        </div>
      </div>
    );
  }

  return (
    <div className={styles.wrapper}>
      <div className={styles.header}>
        <div>
          <p className={styles.title}>All Profiles</p>
          <p className={styles.count}>
            {instances.length} profile{instances.length === 1 ? "" : "s"}
          </p>
        </div>
        <RetroButton variant="secondary" onClick={openNewProfileDialog}>
          + New Profile
        </RetroButton>
      </div>

      {instances.length === 0 ? (
        <div className={styles.state}>
          <p>No profiles yet.</p>
          <RetroButton variant="primary" onClick={openNewProfileDialog}>
            Create your first profile
          </RetroButton>
        </div>
      ) : (
        <div className={styles.grid}>
          {instances.map((instance) => (
            <ProfileListItem
              key={instance.id}
              instance={instance}
              selected={instance.id === selectedId}
              onSelect={() => selectInstance(instance.id)}
              onToggleFavorite={() => void toggleFavorite(instance.id)}
              extraActions={
                <>
                  <button
                    type="button"
                    className={styles.iconButton}
                    aria-label={`Rename ${instance.name}`}
                    title="Rename"
                    onClick={() => setRenaming(instance)}
                  >
                    <Icon name="pencil" size={14} />
                  </button>
                  <button
                    type="button"
                    className={[styles.iconButton, styles.iconButtonDanger].join(" ")}
                    aria-label={`Delete ${instance.name}`}
                    title="Delete"
                    onClick={() => {
                      setDeleteError(null);
                      setDeleting(instance);
                    }}
                  >
                    <Icon name="close" size={14} />
                  </button>
                </>
              }
            />
          ))}
        </div>
      )}

      {renaming ? (
        <RenameProfileDialog instance={renaming} onClose={() => setRenaming(null)} />
      ) : null}

      {deleting ? (
        <ConfirmDialog
          heading="Delete Profile"
          message={
            <>
              Delete <strong>{deleting.name}</strong>? This permanently removes its mods, saves,
              resource packs, shader packs and screenshots. This cannot be undone.
              {deleteError ? <span className={styles.deleteError}>{deleteError}</span> : null}
            </>
          }
          confirmLabel="Delete"
          danger
          isBusy={isDeleting}
          onConfirm={() => void handleConfirmDelete()}
          onCancel={() => setDeleting(null)}
        />
      ) : null}
    </div>
  );
}
