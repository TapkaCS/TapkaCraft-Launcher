import { Icon, type IconName } from "@/components/Icon/Icon";
import type { DashboardTab } from "@/state/uiStore";

import styles from "./Sidebar.module.css";

const ITEMS: { id: DashboardTab; label: string; icon: IconName }[] = [
  { id: "play", label: "Play", icon: "play" },
  { id: "friends", label: "Friends", icon: "users" },
  { id: "lan", label: "LAN", icon: "wifi" },
  { id: "modrinth", label: "Mods", icon: "compass" },
  { id: "modpacks", label: "Modpacks", icon: "box" },
  { id: "profiles", label: "Profiles", icon: "grid" },
  { id: "downloads", label: "Downloads", icon: "download" },
  { id: "settings", label: "Settings", icon: "sliders" },
];

interface SidebarProps {
  active: DashboardTab;
  onChange: (tab: DashboardTab) => void;
}

export function Sidebar({ active, onChange }: SidebarProps) {
  return (
    <nav className={styles.sidebar} aria-label="Launcher sections">
      {ITEMS.map((item) => (
        <button
          key={item.id}
          type="button"
          className={[styles.item, active === item.id ? styles.active : ""]
            .filter(Boolean)
            .join(" ")}
          aria-current={active === item.id ? "page" : undefined}
          onClick={() => onChange(item.id)}
        >
          <Icon name={item.icon} size={18} />
          <span>{item.label}</span>
        </button>
      ))}
    </nav>
  );
}
