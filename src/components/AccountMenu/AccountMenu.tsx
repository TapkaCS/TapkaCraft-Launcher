import { useEffect, useRef, useState } from "react";

import { Icon } from "@/components/Icon/Icon";
import type { MinecraftAccount } from "@/types/account";

import styles from "./AccountMenu.module.css";

interface AccountMenuProps {
  account: MinecraftAccount;
  onSignOut: () => void;
}

export function AccountMenu({ account, onSignOut }: AccountMenuProps) {
  const [open, setOpen] = useState(false);
  const [infoOpen, setInfoOpen] = useState(false);
  const wrapperRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handlePointerDown(event: MouseEvent) {
      if (wrapperRef.current && !wrapperRef.current.contains(event.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", handlePointerDown);
    return () => document.removeEventListener("mousedown", handlePointerDown);
  }, []);

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
                <strong>{account.id}</strong>
              </p>
            </div>
          ) : null}

          <button
            type="button"
            role="menuitem"
            className={styles.menuItem}
            disabled
            title="Available once multi-account support ships (Phase 4)"
          >
            Switch account
            <span className={styles.menuHint}>Coming in a later phase</span>
          </button>

          <div className={styles.separator} />

          <button
            type="button"
            role="menuitem"
            className={[styles.menuItem, styles.danger].join(" ")}
            onClick={() => {
              setOpen(false);
              onSignOut();
            }}
          >
            Sign out
          </button>
        </div>
      ) : null}
    </div>
  );
}
