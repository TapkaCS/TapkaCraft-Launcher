import { useEffect } from "react";

import { Icon } from "@/components/Icon/Icon";
import { InstallProgressBar } from "@/components/InstallProgressBar/InstallProgressBar";
import { RetroButton } from "@/components/RetroButton/RetroButton";
import { useInstanceStore } from "@/state/instanceStore";
import { useModrinthStore } from "@/state/modrinthStore";
import { LOADER_LABELS } from "@/types/instance";
import type { SearchHit } from "@/types/modrinth";

import styles from "./ModrinthTab.module.css";

function formatDownloads(value: number): string {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}k`;
  return String(value);
}

export function ModrinthTab() {
  const instances = useInstanceStore((state) => state.instances);
  const selectedId = useInstanceStore((state) => state.selectedInstanceId);
  const selectedInstance = instances.find((instance) => instance.id === selectedId) ?? null;
  const selectedInstanceId = selectedInstance?.id;
  const canInstall = selectedInstance?.loader.type === "fabric";

  const query = useModrinthStore((state) => state.query);
  const setQuery = useModrinthStore((state) => state.setQuery);
  const results = useModrinthStore((state) => state.results);
  const searchStatus = useModrinthStore((state) => state.searchStatus);
  const searchError = useModrinthStore((state) => state.searchError);
  const search = useModrinthStore((state) => state.search);
  const installedMods = useModrinthStore((state) => state.installedMods);
  const loadInstalledMods = useModrinthStore((state) => state.loadInstalledMods);
  const installingProjectId = useModrinthStore((state) => state.installingProjectId);
  const installProgress = useModrinthStore((state) => state.installProgress);
  const installError = useModrinthStore((state) => state.installError);
  const skippedDependencies = useModrinthStore((state) => state.skippedDependencies);
  const install = useModrinthStore((state) => state.install);

  useEffect(() => {
    if (selectedInstanceId) {
      void loadInstalledMods(selectedInstanceId);
    }
  }, [selectedInstanceId, loadInstalledMods]);

  if (!selectedInstance) {
    return (
      <div className={styles.wrapper}>
        <p className={styles.state}>
          Select or create a profile first — mods are installed into a specific profile.
        </p>
      </div>
    );
  }

  const installedProjectIds = new Set(
    Object.values(installedMods?.mods ?? {}).map((mod) => mod.projectId),
  );

  function handleSearch() {
    if (!selectedInstance) return;
    void search(selectedInstance.loader.type, selectedInstance.minecraftVersion);
  }

  return (
    <div className={styles.wrapper}>
      <div className={styles.header}>
        <div>
          <p className={styles.title}>Browse Modrinth</p>
          <p className={styles.subtitle}>
            For <strong>{selectedInstance.name}</strong> ({selectedInstance.minecraftVersion},{" "}
            {LOADER_LABELS[selectedInstance.loader.type]})
          </p>
        </div>
        <form
          className={styles.searchForm}
          onSubmit={(event) => {
            event.preventDefault();
            handleSearch();
          }}
        >
          <input
            type="text"
            className={styles.searchInput}
            placeholder="Search mods…"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
          <RetroButton type="submit" variant="secondary" disabled={searchStatus === "loading"}>
            Search
          </RetroButton>
        </form>
      </div>

      {!canInstall ? (
        <p className={styles.notice}>
          Only Fabric profiles support installing mods so far. You can still browse, but Install is
          disabled for this profile.
        </p>
      ) : null}
      {installError ? (
        <p className={[styles.notice, styles.errorText].join(" ")}>{installError}</p>
      ) : null}
      {skippedDependencies.length > 0 ? (
        <p className={styles.notice}>
          Installed, but {skippedDependencies.length} dependenc
          {skippedDependencies.length === 1 ? "y" : "ies"} had no compatible build and{" "}
          {skippedDependencies.length === 1 ? "was" : "were"} skipped:{" "}
          {skippedDependencies.join(", ")}
        </p>
      ) : null}
      {installingProjectId && installProgress ? (
        <InstallProgressBar progress={installProgress} label="Installing mod…" />
      ) : null}

      <div className={styles.results}>
        {searchStatus === "loading" ? (
          <p className={styles.state}>Searching…</p>
        ) : searchStatus === "error" ? (
          <p className={[styles.state, styles.errorText].join(" ")}>{searchError}</p>
        ) : results.length === 0 ? (
          <p className={styles.state}>
            {query ? "No mods found." : "Search Modrinth to find mods for this profile."}
          </p>
        ) : (
          results.map((hit) => (
            <ModResultRow
              key={hit.project_id}
              hit={hit}
              installed={installedProjectIds.has(hit.project_id)}
              installing={installingProjectId === hit.project_id}
              disabled={!canInstall || installingProjectId !== null}
              onInstall={() =>
                void install(
                  selectedInstance.id,
                  hit.project_id,
                  selectedInstance.loader.type,
                  selectedInstance.minecraftVersion,
                )
              }
            />
          ))
        )}
      </div>
    </div>
  );
}

function ModResultRow({
  hit,
  installed,
  installing,
  disabled,
  onInstall,
}: {
  hit: SearchHit;
  installed: boolean;
  installing: boolean;
  disabled: boolean;
  onInstall: () => void;
}) {
  return (
    <div className={styles.card}>
      {hit.icon_url ? (
        <img className={styles.icon} src={hit.icon_url} alt="" />
      ) : (
        <div className={styles.iconFallback}>
          <Icon name="box" size={22} />
        </div>
      )}
      <div className={styles.info}>
        <p className={styles.cardTitle}>{hit.title}</p>
        <p className={styles.description}>{hit.description}</p>
        <p className={styles.stats}>&#8595; {formatDownloads(hit.downloads)} downloads</p>
      </div>
      <RetroButton
        variant={installed ? "secondary" : "primary"}
        disabled={disabled}
        onClick={onInstall}
      >
        {installing ? "Installing…" : installed ? "Installed ✓" : "Install"}
      </RetroButton>
    </div>
  );
}
