import { useEffect } from "react";

import { Icon } from "@/components/Icon/Icon";
import { InstallProgressBar } from "@/components/InstallProgressBar/InstallProgressBar";
import { RetroButton } from "@/components/RetroButton/RetroButton";
import { RetroPanel } from "@/components/RetroPanel/RetroPanel";
import { RetroSelect } from "@/components/RetroSelect/RetroSelect";
import { useInstanceStore } from "@/state/instanceStore";
import { useLanStore } from "@/state/lanStore";
import { useLaunchStore } from "@/state/launchStore";
import type { LanGame } from "@/types/lan";

import styles from "./LanTab.module.css";

/**
 * Minecraft's own "Open to LAN" feature broadcasts a small UDP multicast
 * packet on the local network - no server, no account beyond the one
 * already playing. This tab just listens for it, the same way Minecraft's
 * own Multiplayer screen does, so it's real (unlike Friends/Party, see
 * `ComingSoonPanel`) even though it only works with people on the same
 * Wi-Fi/network.
 */
export function LanTab() {
  const games = useLanStore((state) => state.games);
  const status = useLanStore((state) => state.status);
  const error = useLanStore((state) => state.error);
  const start = useLanStore((state) => state.start);
  const stop = useLanStore((state) => state.stop);

  useEffect(() => {
    void start();
    return () => void stop();
    // Runs once on mount/unmount only - start/stop are stable zustand
    // actions, and re-running per render would restart the listener.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const instances = useInstanceStore((state) => state.instances);
  const selectedId = useInstanceStore((state) => state.selectedInstanceId);
  const selectInstance = useInstanceStore((state) => state.selectInstance);

  const launchingInstanceId = useLaunchStore((state) => state.launchingInstanceId);
  const launchPhase = useLaunchStore((state) => state.phase);
  const launchInstallProgress = useLaunchStore((state) => state.installProgress);
  const preparingLabel = useLaunchStore((state) => state.preparingLabel);
  const launchError = useLaunchStore((state) => state.error);
  const launch = useLaunchStore((state) => state.launch);

  const isBusy = launchingInstanceId !== null;

  function handleJoin(game: LanGame) {
    if (!selectedId || isBusy) return;
    void launch(selectedId, { host: game.host, port: game.port });
  }

  return (
    <div className={styles.layout}>
      <RetroPanel
        heading="LAN Games"
        headerExtra={
          <span className={styles.statusPill} data-status={status}>
            <Icon name="wifi" size={13} />
            {status === "listening" ? "Listening" : status === "error" ? "Not listening" : "Idle"}
          </span>
        }
      >
        <p className={styles.description}>
          Finds worlds opened with Minecraft&apos;s own &quot;Open to LAN&quot; button on your local
          network - no TapkaCraft account or server involved, same as vanilla&apos;s own Multiplayer
          screen.
        </p>

        <div className={styles.joinAsRow}>
          <RetroSelect
            label="Join as"
            value={selectedId ?? ""}
            onChange={(event) => selectInstance(event.target.value)}
            disabled={instances.length === 0}
          >
            {instances.length === 0 ? <option value="">No profiles yet</option> : null}
            {instances.map((instance) => (
              <option key={instance.id} value={instance.id}>
                {instance.name} ({instance.minecraftVersion})
              </option>
            ))}
          </RetroSelect>
        </div>

        {error ? <p className={styles.error}>{error}</p> : null}
        {launchPhase === "error" && launchError ? (
          <p className={styles.error}>{launchError}</p>
        ) : null}
        {isBusy && launchPhase === "preparing" && launchInstallProgress ? (
          <InstallProgressBar
            progress={launchInstallProgress}
            label={preparingLabel ?? "Preparing…"}
          />
        ) : null}

        {games.length === 0 ? (
          <div className={styles.empty}>
            <div className={styles.emptyIcon}>
              <Icon name="wifi" size={28} />
            </div>
            <p className={styles.emptyTitle}>No LAN games found yet</p>
            <p className={styles.emptyDescription}>
              Ask a friend on the same network to open their world to LAN from Minecraft&apos;s
              pause menu - it&apos;ll show up here automatically.
            </p>
          </div>
        ) : (
          <ul className={styles.gameList}>
            {games.map((game) => {
              const isJoiningThis =
                isBusy && launchingInstanceId === selectedId && launchPhase !== "idle";
              return (
                <li key={`${game.host}:${game.port}`} className={styles.gameCard}>
                  <div className={styles.gameInfo}>
                    <span className={styles.gameMotd}>{game.motd || "A Minecraft World"}</span>
                    <span className={styles.gameAddress}>
                      {game.host}:{game.port}
                    </span>
                  </div>
                  <RetroButton
                    variant="primary"
                    icon={<Icon name="play" size={16} />}
                    disabled={!selectedId || isBusy}
                    title={
                      !selectedId
                        ? "Create a profile first"
                        : `Join ${game.motd || "this world"} as the selected profile`
                    }
                    onClick={() => handleJoin(game)}
                  >
                    {isJoiningThis ? (launchPhase === "running" ? "Playing…" : "Joining…") : "Join"}
                  </RetroButton>
                </li>
              );
            })}
          </ul>
        )}
      </RetroPanel>
    </div>
  );
}
