import { useState } from "react";

import { AccountsSection } from "./sections/AccountsSection";
import { AdvancedSection } from "./sections/AdvancedSection";
import { AppearanceSection } from "./sections/AppearanceSection";
import { DownloadsSection } from "./sections/DownloadsSection";
import { GeneralSection } from "./sections/GeneralSection";
import { JavaSection } from "./sections/JavaSection";
import { MinecraftSection } from "./sections/MinecraftSection";
import styles from "./SettingsScreen.module.css";

const CATEGORIES = [
  { id: "general", label: "General" },
  { id: "minecraft", label: "Minecraft" },
  { id: "java", label: "Java" },
  { id: "accounts", label: "Accounts" },
  { id: "downloads", label: "Downloads" },
  { id: "appearance", label: "Appearance" },
  { id: "advanced", label: "Advanced" },
] as const;

type SettingsCategory = (typeof CATEGORIES)[number]["id"];

export function SettingsScreen() {
  const [category, setCategory] = useState<SettingsCategory>("general");

  return (
    <div className={styles.layout}>
      <nav className={styles.nav} aria-label="Settings categories">
        {CATEGORIES.map((item) => (
          <button
            key={item.id}
            type="button"
            className={[styles.navItem, category === item.id ? styles.navItemActive : ""]
              .filter(Boolean)
              .join(" ")}
            onClick={() => setCategory(item.id)}
          >
            {item.label}
          </button>
        ))}
      </nav>

      <div className={styles.content}>
        {category === "general" ? <GeneralSection /> : null}
        {category === "minecraft" ? <MinecraftSection /> : null}
        {category === "java" ? <JavaSection /> : null}
        {category === "accounts" ? <AccountsSection /> : null}
        {category === "downloads" ? <DownloadsSection /> : null}
        {category === "appearance" ? <AppearanceSection /> : null}
        {category === "advanced" ? <AdvancedSection /> : null}
      </div>
    </div>
  );
}
