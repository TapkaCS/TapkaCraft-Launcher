import { Icon, type IconName } from "@/components/Icon/Icon";

import styles from "./ComingSoonPanel.module.css";

interface ComingSoonPanelProps {
  icon: IconName;
  title: string;
  description: string;
}

/**
 * Used for dashboard tabs whose backend doesn't exist yet (Modrinth,
 * Modpacks). Deliberately not a fake search box or install button - just an
 * honest placeholder that says what's coming and why it isn't here now.
 */
export function ComingSoonPanel({ icon, title, description }: ComingSoonPanelProps) {
  return (
    <div className={styles.panel}>
      <div className={styles.iconWrap}>
        <Icon name={icon} size={26} />
      </div>
      <p className={styles.title}>{title}</p>
      <p className={styles.description}>{description}</p>
      <span className={styles.badge}>Coming in a later phase</span>
    </div>
  );
}
