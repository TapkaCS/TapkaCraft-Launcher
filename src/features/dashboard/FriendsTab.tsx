import { useState } from "react";

import { Icon, type IconName } from "@/components/Icon/Icon";

import { ComingSoonPanel } from "./ComingSoonPanel";
import styles from "./FriendsTab.module.css";

type FriendsSection = "friends" | "party" | "join";

const SECTIONS: { id: FriendsSection; label: string; icon: IconName }[] = [
  { id: "friends", label: "Friends", icon: "users" },
  { id: "party", label: "Party", icon: "crown" },
  { id: "join", label: "Join a Friend", icon: "userPlus" },
];

const CONTENT: Record<FriendsSection, { icon: IconName; title: string; description: string }> = {
  friends: {
    icon: "users",
    title: "Friends",
    description:
      "Seeing who's online and what they're playing needs a real TapkaCraft server tracking presence - this launcher only talks to Mojang, Microsoft and Modrinth today. That's a hosting commitment worth doing right, not faking.",
  },
  party: {
    icon: "crown",
    title: "Party & Squad Lobby",
    description:
      "Grouping up with friends needs that same presence system plus real-time lobby state, and voice chat specifically is its own large undertaking (signaling, relay servers) beyond what this launcher can bolt on. Not something to fake with buttons that don't actually connect anyone.",
  },
  join: {
    icon: "userPlus",
    title: "Join a Friend",
    description:
      "Jumping straight into a friend's game wherever they are needs their server address from that same presence system. On the same local network, though, this is already real today - see the LAN tab, which finds and joins Minecraft's own \"Open to LAN\" games without needing any of that.",
  },
};

/**
 * Friends/Party/Join-a-Friend all need the same thing this launcher has
 * never had: a real TapkaCraft server tracking who's online and grouped
 * with whom. Kept as three distinct, honestly-labeled panels (matching the
 * three separate features asked for) rather than one vague placeholder,
 * so it's clear each was actually considered - see ComingSoonPanel's own
 * doc comment for why a fake presence UI isn't the answer either.
 */
export function FriendsTab() {
  const [section, setSection] = useState<FriendsSection>("friends");
  const content = CONTENT[section];

  return (
    <div className={styles.layout}>
      <div className={styles.sectionRow}>
        {SECTIONS.map((item) => (
          <button
            key={item.id}
            type="button"
            className={[styles.sectionButton, section === item.id ? styles.sectionActive : ""]
              .filter(Boolean)
              .join(" ")}
            onClick={() => setSection(item.id)}
          >
            <Icon name={item.icon} size={16} />
            {item.label}
          </button>
        ))}
      </div>
      <ComingSoonPanel
        icon={content.icon}
        title={content.title}
        description={content.description}
      />
    </div>
  );
}
