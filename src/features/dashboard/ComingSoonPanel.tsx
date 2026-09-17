import { Icon, type IconName } from "@/components/Icon/Icon";

import styles from "./ComingSoonPanel.module.css";

interface ComingSoonPanelProps {
  icon: IconName;
  title: string;
  description: string;
}

/**
 * Used for dashboard tabs whose backend doesn't exist yet (Friends, LAN
 * party coordination) - deliberately not a fake friends list or party UI
 * that looks live but does nothing. Friends/Party specifically need a real
 * TapkaCraft server (presence, accounts beyond Microsoft sign-in) that this
 * project has never had; that's a hosting/infrastructure commitment, not
 * just more frontend code, so it's named honestly here instead of faked.
 */
export function ComingSoonPanel({ icon, title, description }: ComingSoonPanelProps) {
  return (
    <div className={styles.panel}>
      <div className={styles.iconWrap}>
        <Icon name={icon} size={28} />
      </div>
      <p className={styles.title}>{title}</p>
      <p className={styles.description}>{description}</p>
      <span className={styles.badge}>Coming in a later phase</span>
    </div>
  );
}
