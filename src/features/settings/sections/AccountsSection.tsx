import { useEffect, useState } from "react";

import { RetroButton } from "@/components/RetroButton/RetroButton";
import { listAccounts, removeAccount, switchAccount } from "@/lib/api/accounts";
import { isTauri } from "@/lib/tauri";
import { useAuthStore } from "@/state/authStore";
import type { MinecraftAccount } from "@/types/account";

import styles from "./SettingsSections.module.css";

function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export function AccountsSection() {
  const authState = useAuthStore((state) => state.state);
  const signOut = useAuthStore((state) => state.signOut);
  const signIn = useAuthStore((state) => state.signIn);
  const restoreSession = useAuthStore((state) => state.restoreSession);

  const activeAccount: MinecraftAccount | null =
    authState.status === "authenticated" || authState.status === "refreshing"
      ? authState.account
      : null;

  const [accounts, setAccounts] = useState<MinecraftAccount[]>([]);
  const [accountsError, setAccountsError] = useState<string | null>(null);
  const [busyUuid, setBusyUuid] = useState<string | null>(null);

  useEffect(() => {
    if (!isTauri) return;
    let cancelled = false;
    listAccounts()
      .then((result) => {
        if (!cancelled) setAccounts(result);
      })
      .catch((err) => {
        if (!cancelled) setAccountsError(errorMessage(err));
      });
    // Re-list whenever the active session changes (sign in/out/switch), so
    // this stays in sync without its own polling.
    return () => {
      cancelled = true;
    };
  }, [authState.status, activeAccount?.minecraftUuid]);

  async function handleSwitch(uuid: string) {
    setAccountsError(null);
    setBusyUuid(uuid);
    try {
      await switchAccount(uuid);
      await restoreSession();
    } catch (err) {
      setAccountsError(errorMessage(err));
    } finally {
      setBusyUuid(null);
    }
  }

  async function handleForget(uuid: string) {
    setAccountsError(null);
    setBusyUuid(uuid);
    try {
      await removeAccount(uuid);
      setAccounts((current) => current.filter((account) => account.minecraftUuid !== uuid));
      if (activeAccount?.minecraftUuid === uuid) {
        await signOut();
      }
    } catch (err) {
      setAccountsError(errorMessage(err));
    } finally {
      setBusyUuid(null);
    }
  }

  return (
    <div className={styles.section}>
      <h2 className={styles.sectionTitle}>Accounts</h2>

      {!isTauri ? (
        <div className={styles.emptyState}>Managing accounts requires the desktop app.</div>
      ) : accounts.length === 0 ? (
        <div className={styles.emptyState}>No accounts signed in yet.</div>
      ) : (
        <ul className={styles.runtimeList}>
          {accounts.map((account) => {
            const isActive = account.minecraftUuid === activeAccount?.minecraftUuid;
            return (
              <li key={account.minecraftUuid} className={styles.runtimeItem}>
                <div className={styles.accountRow}>
                  <div className={styles.rowLabel}>
                    <strong>{account.username}</strong>
                    <span>
                      {account.minecraftUuid} &middot; Microsoft Account
                      {isActive ? " · Active" : ""}
                    </span>
                  </div>
                  <div className={styles.rowControl}>
                    {!isActive ? (
                      <RetroButton
                        variant="secondary"
                        disabled={busyUuid === account.minecraftUuid}
                        onClick={() => void handleSwitch(account.minecraftUuid)}
                      >
                        {busyUuid === account.minecraftUuid ? "Switching…" : "Switch to"}
                      </RetroButton>
                    ) : null}
                    <RetroButton
                      variant="secondary"
                      disabled={busyUuid === account.minecraftUuid}
                      onClick={() => void handleForget(account.minecraftUuid)}
                    >
                      Forget
                    </RetroButton>
                  </div>
                </div>
              </li>
            );
          })}
        </ul>
      )}

      {accountsError ? <p className={styles.hint}>{accountsError}</p> : null}

      <div className={styles.row}>
        <div className={styles.rowLabel}>
          <strong>Add another account</strong>
          <span>Multiple Microsoft accounts can be signed in at once.</span>
        </div>
        <div className={styles.rowControl}>
          <RetroButton variant="secondary" onClick={() => void signIn()}>
            Add Account
          </RetroButton>
        </div>
      </div>
    </div>
  );
}
