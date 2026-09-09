import { useEffect, useRef, useState } from "react";

import { Icon } from "@/components/Icon/Icon";
import { listAccounts, removeAccount, switchAccount } from "@/lib/api/accounts";
import { useAuthStore } from "@/state/authStore";
import type { MinecraftAccount } from "@/types/account";

import styles from "./AccountMenu.module.css";

interface AccountMenuProps {
  account: MinecraftAccount;
}

function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export function AccountMenu({ account }: AccountMenuProps) {
  const [open, setOpen] = useState(false);
  const [infoOpen, setInfoOpen] = useState(false);
  const [otherAccounts, setOtherAccounts] = useState<MinecraftAccount[]>([]);
  const [switchingUuid, setSwitchingUuid] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const wrapperRef = useRef<HTMLDivElement>(null);

  const signOut = useAuthStore((state) => state.signOut);
  const signIn = useAuthStore((state) => state.signIn);
  const restoreSession = useAuthStore((state) => state.restoreSession);

  useEffect(() => {
    function handlePointerDown(event: MouseEvent) {
      if (wrapperRef.current && !wrapperRef.current.contains(event.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", handlePointerDown);
    return () => document.removeEventListener("mousedown", handlePointerDown);
  }, []);

  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    listAccounts()
      .then((accounts) => {
        if (!cancelled) {
          setOtherAccounts(accounts.filter((a) => a.minecraftUuid !== account.minecraftUuid));
        }
      })
      .catch(() => {
        if (!cancelled) setOtherAccounts([]);
      });
    return () => {
      cancelled = true;
    };
  }, [open, account.minecraftUuid]);

  async function handleSwitch(uuid: string) {
    setActionError(null);
    setSwitchingUuid(uuid);
    try {
      await switchAccount(uuid);
      await restoreSession();
      setOpen(false);
    } catch (err) {
      setActionError(errorMessage(err));
    } finally {
      setSwitchingUuid(null);
    }
  }

  async function handleForget(uuid: string) {
    setActionError(null);
    try {
      await removeAccount(uuid);
      setOtherAccounts((accounts) => accounts.filter((a) => a.minecraftUuid !== uuid));
    } catch (err) {
      setActionError(errorMessage(err));
    }
  }

  return (
    <div className={styles.wrapper} ref={wrapperRef}>
      <button
        type="button"
        className={styles.trigger}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((value) => !value)}
      >
        <span className={styles.avatar} aria-hidden="true">
          {account.username.charAt(0).toUpperCase()}
        </span>
        <span className={styles.identity}>
          <span className={styles.username}>{account.username}</span>
          <span className={styles.provider}>Microsoft Account</span>
        </span>
        <Icon name="chevronDown" size={14} />
      </button>

      {open ? (
        <div className={styles.menu} role="menu">
          <button
            type="button"
            role="menuitem"
            className={styles.menuItem}
            onClick={() => setInfoOpen((value) => !value)}
          >
            Account information
          </button>

          {infoOpen ? (
            <div className={styles.info}>
              <p>
                <span>Username</span>
                <strong>{account.username}</strong>
              </p>
              <p>
                <span>UUID</span>
                <strong>{account.minecraftUuid}</strong>
              </p>
            </div>
          ) : null}

          {actionError ? <p className={styles.menuError}>{actionError}</p> : null}

          {otherAccounts.length > 0 ? (
            <>
              <div className={styles.separator} />
              {otherAccounts.map((other) => (
                <div key={other.minecraftUuid} className={styles.otherAccountRow}>
                  <button
                    type="button"
                    role="menuitem"
                    className={styles.menuItem}
                    disabled={switchingUuid === other.minecraftUuid}
                    onClick={() => void handleSwitch(other.minecraftUuid)}
                  >
                    {switchingUuid === other.minecraftUuid
                      ? "Switching…"
                      : `Switch to ${other.username}`}
                  </button>
                  <button
                    type="button"
                    className={styles.forgetButton}
                    title={`Forget ${other.username}`}
                    aria-label={`Forget ${other.username}`}
                    onClick={() => void handleForget(other.minecraftUuid)}
                  >
                    <Icon name="close" size={12} />
                  </button>
                </div>
              ))}
            </>
          ) : null}

          <div className={styles.separator} />

          <button
            type="button"
            role="menuitem"
            className={styles.menuItem}
            onClick={() => {
              setOpen(false);
              void signIn();
            }}
          >
            Add another account
          </button>

          <div className={styles.separator} />

          <button
            type="button"
            role="menuitem"
            className={[styles.menuItem, styles.danger].join(" ")}
            onClick={() => {
              setOpen(false);
              void signOut();
            }}
          >
            Sign out
          </button>
        </div>
      ) : null}
    </div>
  );
}
