import { useEffect, useState } from "react";

import { Icon } from "@/components/Icon/Icon";
import { InstallProgressBar } from "@/components/InstallProgressBar/InstallProgressBar";
import { RetroButton } from "@/components/RetroButton/RetroButton";
import { RetroSelect } from "@/components/RetroSelect/RetroSelect";
import { getVersionManifest } from "@/lib/api/versions";
import { isTauri } from "@/lib/tauri";
import { useModpackStore } from "@/state/modpackStore";
import type { SearchHit } from "@/types/modrinth";

import styles from "./ModpacksTab.module.css";

const FALLBACK_MINECRAFT_VERSIONS = [
  "1.21.1",
  "1.20.4",
  "1.20.1",
  "1.19.4",
  "1.18.2",
  "1.16.5",
  "1.8.9",
];

const LOADERS: { value: string; label: string }[] = [
  { value: "fabric", label: "Fabric" },
  { value: "quilt", label: "Quilt" },
  { value: "forge", label: "Forge" },
  { value: "neoforge", label: "NeoForge" },
];

function formatDownloads(value: number): string {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}k`;
  return String(value);
}

export function ModpacksTab() {
  const [versions, setVersions] = useState<string[]>(FALLBACK_MINECRAFT_VERSIONS);
  const [gameVersion, setGameVersion] = useState(FALLBACK_MINECRAFT_VERSIONS[0]);
  const [loader, setLoader] = useState(LOADERS[0].value);

  const query = useModpackStore((state) => state.query);
  const setQuery = useModpackStore((state) => state.setQuery);
  const results = useModpackStore((state) => state.results);
  const searchStatus = useModpackStore((state) => state.searchStatus);
  const searchError = useModpackStore((state) => state.searchError);
  const search = useModpackStore((state) => state.search);
  const installingProjectId = useModpackStore((state) => state.installingProjectId);
  const isImportingFile = useModpackStore((state) => state.isImportingFile);
  const installProgress = useModpackStore((state) => state.installProgress);
  const installError = useModpackStore((state) => state.installError);
  const install = useModpackStore((state) => state.install);
  const importFromFile = useModpackStore((state) => state.importFromFile);

  useEffect(() => {
    if (!isTauri) return;
    let cancelled = false;
    getVersionManifest()
      .then((summaries) => {
        if (cancelled || summaries.length === 0) return;
        const ids = summaries.map((summary) => summary.id);
        setVersions(ids);
        setGameVersion(ids[0]);
      })
      .catch(() => {
        // Real Mojang manifest unreachable - keep the fallback list.
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const busy = installingProjectId !== null || isImportingFile;

  function handleSearch() {
    void search(loader, gameVersion);
  }

  return (
    <div className={styles.wrapper}>
      <div className={styles.header}>
        <div>
          <p className={styles.title}>Browse Modpacks</p>
          <p className={styles.subtitle}>Importing creates a brand new profile for the pack.</p>
        </div>
        <RetroButton variant="secondary" disabled={busy} onClick={() => void importFromFile()}>
          {isImportingFile ? "Importing…" : "Import from file…"}
        </RetroButton>
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
          placeholder="Search modpacks…"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
        <RetroSelect
          aria-label="Minecraft version"
          value={gameVersion}
          onChange={(event) => setGameVersion(event.target.value)}
        >
          {versions.map((version) => (
            <option key={version} value={version}>
              {version}
            </option>
          ))}
        </RetroSelect>
        <RetroSelect
          aria-label="Loader"
          value={loader}
          onChange={(event) => setLoader(event.target.value)}
        >
          {LOADERS.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </RetroSelect>
        <RetroButton type="submit" variant="secondary" disabled={searchStatus === "loading"}>
          Search
        </RetroButton>
      </form>

      {loader !== "fabric" ? (
        <p className={styles.notice}>
          Only Fabric modpacks can be installed so far. You can still browse{" "}
          {LOADERS.find((option) => option.value === loader)?.label} packs, but Install will be
          disabled for them.
        </p>
      ) : null}
      {installError ? (
        <p className={[styles.notice, styles.errorText].join(" ")}>{installError}</p>
      ) : null}
      {busy && installProgress ? (
        <InstallProgressBar progress={installProgress} label="Installing modpack…" />
      ) : null}

      <div className={styles.results}>
        {searchStatus === "loading" ? (
          <p className={styles.state}>Searching…</p>
        ) : searchStatus === "error" ? (
          <p className={[styles.state, styles.errorText].join(" ")}>{searchError}</p>
        ) : results.length === 0 ? (
          <p className={styles.state}>
            {query ? "No modpacks found." : "Search Modrinth to find a modpack to play."}
          </p>
        ) : (
          results.map((hit) => (
            <ModpackResultRow
              key={hit.project_id}
              hit={hit}
              installing={installingProjectId === hit.project_id}
              disabled={busy || loader !== "fabric"}
              onInstall={() => void install(hit.project_id, loader, gameVersion)}
            />
          ))
        )}
      </div>
    </div>
  );
}

function ModpackResultRow({
  hit,
  installing,
  disabled,
  onInstall,
}: {
  hit: SearchHit;
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
      <RetroButton variant="primary" disabled={disabled} onClick={onInstall}>
        {installing ? "Installing…" : "Install"}
      </RetroButton>
    </div>
  );
}
