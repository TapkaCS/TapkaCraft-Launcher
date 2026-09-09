import { ProfileListItem } from "@/components/ProfileListItem/ProfileListItem";
import { RetroButton } from "@/components/RetroButton/RetroButton";
import { useInstanceStore } from "@/state/instanceStore";
import { useUiStore } from "@/state/uiStore";

import styles from "./ProfilesTab.module.css";

export function ProfilesTab() {
  const instances = useInstanceStore((state) => state.instances);
  const selectedId = useInstanceStore((state) => state.selectedInstanceId);
  const selectInstance = useInstanceStore((state) => state.selectInstance);
  const toggleFavorite = useInstanceStore((state) => state.toggleFavorite);
  const openNewProfileDialog = useUiStore((state) => state.openNewProfileDialog);

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

      <div className={styles.grid}>
        {instances.map((instance) => (
          <ProfileListItem
            key={instance.id}
            instance={instance}
            selected={instance.id === selectedId}
            onSelect={() => selectInstance(instance.id)}
            onToggleFavorite={() => toggleFavorite(instance.id)}
          />
        ))}
      </div>
    </div>
  );
}
