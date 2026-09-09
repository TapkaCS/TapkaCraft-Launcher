import { useState } from "react";

import { Icon } from "@/components/Icon/Icon";
import { ModCard } from "@/components/ModCard/ModCard";
import { ProfileListItem } from "@/components/ProfileListItem/ProfileListItem";
import { RetroButton } from "@/components/RetroButton/RetroButton";
import { RetroPanel } from "@/components/RetroPanel/RetroPanel";
import { RetroSelect } from "@/components/RetroSelect/RetroSelect";
import { MOCK_FEATURED_MODS, MOCK_NEWS_SLIDES } from "@/lib/mock/mockData";
import { useInstanceStore } from "@/state/instanceStore";
import { useUiStore } from "@/state/uiStore";

import styles from "./PlayTab.module.css";

export function PlayTab() {
  const instances = useInstanceStore((state) => state.instances);
  const status = useInstanceStore((state) => state.status);
  const error = useInstanceStore((state) => state.error);
  const selectedId = useInstanceStore((state) => state.selectedInstanceId);
  const selectInstance = useInstanceStore((state) => state.selectInstance);
  const toggleFavorite = useInstanceStore((state) => state.toggleFavorite);
  const loadInstances = useInstanceStore((state) => state.loadInstances);
  const openNewProfileDialog = useUiStore((state) => state.openNewProfileDialog);
  const setActiveTab = useUiStore((state) => state.setActiveTab);

  const slide = MOCK_NEWS_SLIDES[0];

  return (
    <div className={styles.layout}>
      <div className={styles.leftColumn}>
        <RetroPanel
          heading="Profiles"
          headerExtra={
            <button
              type="button"
              className={styles.newProfileButton}
              onClick={openNewProfileDialog}
            >
              + New Profile
            </button>
          }
          noPadding
          className={styles.profilesPanel}
        >
          <div className={styles.profileList}>
            {status === "loading" || status === "idle" ? (
              <p className={styles.sidebarState}>Loading profiles…</p>
            ) : status === "error" ? (
              <div className={styles.sidebarState}>
                <p className={styles.sidebarError}>{error}</p>
                <button
                  type="button"
                  className={styles.retryLink}
                  onClick={() => void loadInstances()}
                >
                  Retry
                </button>
              </div>
            ) : instances.length === 0 ? (
              <p className={styles.sidebarState}>No profiles yet.</p>
            ) : (
              instances.map((instance) => (
                <ProfileListItem
                  key={instance.id}
                  instance={instance}
                  selected={instance.id === selectedId}
                  onSelect={() => selectInstance(instance.id)}
                  onToggleFavorite={() => void toggleFavorite(instance.id)}
                />
              ))
            )}
          </div>
        </RetroPanel>
      </div>

      <div className={styles.mainColumn}>
        <div className={styles.scrollArea}>
          <div className={styles.newsBanner}>
            <p className={styles.newsTitle}>{slide.title}</p>
            <p className={styles.newsSubtitle}>{slide.subtitle}</p>
            <div className={styles.newsDots}>
              {MOCK_NEWS_SLIDES.map((item, index) => (
                <span
                  key={item.id}
                  className={[styles.newsDot, index === 0 ? styles.newsDotActive : ""]
                    .filter(Boolean)
                    .join(" ")}
                />
              ))}
            </div>
          </div>

          <div>
            <div className={styles.sectionHeader}>
              <p className={styles.sectionTitle}>Featured on Modrinth</p>
              <button
                type="button"
                className={styles.browseLink}
                onClick={() => setActiveTab("modrinth")}
              >
                Browse Modrinth &rarr;
              </button>
            </div>
            <div className={styles.modGrid}>
              {MOCK_FEATURED_MODS.map((mod) => (
                <ModCard key={mod.id} mod={mod} />
              ))}
            </div>
          </div>
        </div>

        <PlayBar />
      </div>
    </div>
  );
}

function PlayBar() {
  const instances = useInstanceStore((state) => state.instances);
  const selectedId = useInstanceStore((state) => state.selectedInstanceId);
  const selectInstance = useInstanceStore((state) => state.selectInstance);
  const openInstanceFolder = useInstanceStore((state) => state.openInstanceFolder);
  const [folderError, setFolderError] = useState<string | null>(null);

  async function handleOpenFolder() {
    if (!selectedId) return;
    setFolderError(null);
    try {
      await openInstanceFolder(selectedId);
    } catch (err) {
      setFolderError(err instanceof Error ? err.message : String(err));
    }
  }

  return (
    <div className={styles.playBarWrap}>
      {folderError ? <p className={styles.playBarError}>{folderError}</p> : null}
      <div className={styles.playBar}>
        <RetroSelect
          aria-label="Selected profile"
          className={styles.playBarSelect}
          value={selectedId ?? ""}
          onChange={(event) => selectInstance(event.target.value)}
        >
          {instances.map((instance) => (
            <option key={instance.id} value={instance.id}>
              {instance.name} ({instance.minecraftVersion})
            </option>
          ))}
        </RetroSelect>

        <RetroButton
          variant="primary"
          size="lg"
          className={styles.playButton}
          icon={<Icon name="play" size={20} />}
          disabled
          title="Launching Minecraft needs VersionService, JavaManager and LaunchEngine, which land in Phase 3"
        >
          Play
        </RetroButton>

        <div className={styles.playBarActions}>
          <RetroButton
            variant="secondary"
            icon={<Icon name="folder" size={16} />}
            disabled={!selectedId}
            aria-label="Open instance folder"
            title="Open this profile's folder"
            onClick={() => void handleOpenFolder()}
          />
          <RetroButton
            variant="secondary"
            disabled
            aria-label="More options"
            title="Mods/shaders/screenshots/Smart Upgrade and the rest of the profile context menu land in later phases"
          >
            &#8943;
          </RetroButton>
        </div>
      </div>
    </div>
  );
}
