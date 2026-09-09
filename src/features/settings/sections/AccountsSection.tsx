import { RetroButton } from "@/components/RetroButton/RetroButton";
import { useAuthStore } from "@/state/authStore";

import styles from "./SettingsSections.module.css";

export function AccountsSection() {
  const authState = useAuthStore((state) => state.state);
  const signOut = useAuthStore((state) => state.signOut);

  const account =
    authState.status === "authenticated" || authState.status === "refreshing"
      ? authState.account
      : null;

  return (
    <div className={styles.section}>
      <h2 className={styles.sectionTitle}>Accounts</h2>

      {account ? (
        <div className={styles.row}>
          <div className={styles.rowLabel}>
            <strong>{account.username}</strong>
            <span>{account.id} &middot; Microsoft Account &middot; Active</span>
          </div>
          <div className={styles.rowControl}>
            <RetroButton variant="secondary" onClick={signOut}>
              Sign Out
            </RetroButton>
          </div>
        </div>
      ) : (
        <div className={styles.emptyState}>No account signed in.</div>
      )}

      <div className={styles.row}>
        <div className={styles.rowLabel}>
          <strong>Add another account</strong>
          <span>Multiple Microsoft accounts can be signed in at once.</span>
        </div>
        <div className={styles.rowControl}>
          <RetroButton
            variant="secondary"
            disabled
            title="Multi-account support ships alongside the real Microsoft sign-in flow (Phase 4)"
          >
            Add Account
          </RetroButton>
        </div>
      </div>
    </div>
  );
}
