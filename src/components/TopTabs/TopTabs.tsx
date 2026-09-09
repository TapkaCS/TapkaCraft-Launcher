import { Icon, type IconName } from "@/components/Icon/Icon";
import type { DashboardTab } from "@/state/uiStore";

import styles from "./TopTabs.module.css";

const TABS: { id: DashboardTab; label: string; icon: IconName }[] = [
  { id: "play", label: "Play", icon: "play" },
  { id: "profiles", label: "Profiles", icon: "grid" },
  { id: "modrinth", label: "Modrinth", icon: "compass" },
  { id: "modpacks", label: "Modpacks", icon: "box" },
  { id: "settings", label: "Settings", icon: "sliders" },
];

interface TopTabsProps {
  active: DashboardTab;
  onChange: (tab: DashboardTab) => void;
}

export function TopTabs({ active, onChange }: TopTabsProps) {
  return (
    <nav className={styles.tabs} role="tablist" aria-label="Launcher sections">
      {TABS.map((tab) => (
        <button
          key={tab.id}
          type="button"
          role="tab"
          aria-selected={active === tab.id}
          className={[styles.tab, active === tab.id ? styles.active : ""].filter(Boolean).join(" ")}
          onClick={() => onChange(tab.id)}
        >
          <Icon name={tab.icon} size={16} />
          {tab.label}
        </button>
      ))}
    </nav>
  );
}
