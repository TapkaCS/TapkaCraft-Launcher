/**
 * Mock data used only when there is no Tauri backend to talk to (a plain
 * `vite dev`/`vite preview` browser session, not the actual app) - see
 * `isTauri` in `src/lib/tauri.ts`. `MOCK_INSTANCES` is only
 * `instanceStore.loadInstances`'s dev-mode fallback as of Phase 2 - inside
 * the real app, instances come from `InstanceService` on disk. There's no
 * mock account: Microsoft sign-in (Phase 4) has no meaningful fake
 * equivalent, so `authStore.signIn` just reports "requires the desktop
 * app" outside Tauri instead of pretending to authenticate.
 */

import type { InstanceMeta } from "@/types/instance";

export const MOCK_INSTANCES: InstanceMeta[] = [
  {
    id: "vanilla-1-20-1",
    name: "Vanilla",
    minecraftVersion: "1.20.1",
    loader: { type: "vanilla", version: "1.20.1" },
    java: { memoryMinMb: 1024, memoryMaxMb: 4096 },
    favorite: true,
    playtimeSeconds: 0,
    launchCount: 0,
  },
  {
    id: "fabric-1-20-1",
    name: "Fabric",
    minecraftVersion: "1.20.1",
    loader: { type: "fabric", version: "0.15.7" },
    java: { memoryMinMb: 1024, memoryMaxMb: 4096 },
    favorite: false,
    playtimeSeconds: 0,
    launchCount: 0,
  },
  {
    id: "pvp-1-8-9",
    name: "PvP",
    minecraftVersion: "1.8.9",
    loader: { type: "vanilla", version: "1.8.9" },
    java: { memoryMinMb: 1024, memoryMaxMb: 3072 },
    favorite: false,
    playtimeSeconds: 0,
    launchCount: 0,
  },
  {
    id: "modded-forge",
    name: "Modded",
    minecraftVersion: "1.20.1",
    loader: { type: "forge", version: "47.2.0" },
    java: { memoryMinMb: 2048, memoryMaxMb: 8192 },
    favorite: false,
    playtimeSeconds: 0,
    launchCount: 0,
  },
  {
    id: "skyblock-fabric",
    name: "Skyblock",
    minecraftVersion: "1.19.4",
    loader: { type: "fabric", version: "0.15.7" },
    java: { memoryMinMb: 1024, memoryMaxMb: 4096 },
    favorite: false,
    playtimeSeconds: 0,
    launchCount: 0,
  },
];

export interface FeaturedModCard {
  id: string;
  title: string;
  author: string;
  description: string;
  downloads: number;
  follows: number;
  /** Single-letter/short badge shown in place of a real Modrinth icon. */
  badge: string;
  accent: string;
}

/**
 * Real Sodium/Iris/Lithium/Mod Menu project names, used only as static
 * display copy on the Phase 1 dashboard - no network call reaches Modrinth
 * here. Real search/browse/install lands in Phase 6 via `ModrinthService`.
 */
export const MOCK_FEATURED_MODS: FeaturedModCard[] = [
  {
    id: "sodium",
    title: "Sodium",
    author: "JellySquid",
    description: "A modern rendering engine for Minecraft.",
    downloads: 24_300_000,
    follows: 4200,
    badge: "S",
    accent: "#f97316",
  },
  {
    id: "iris",
    title: "Iris Shaders",
    author: "coderbot",
    description: "High performance shader support for Minecraft.",
    downloads: 18_100_000,
    follows: 3100,
    badge: "I",
    accent: "#a855f7",
  },
  {
    id: "lithium",
    title: "Lithium",
    author: "JellySquid",
    description: "No-compromises gameplay optimization.",
    downloads: 16_800_000,
    follows: 2700,
    badge: "L",
    accent: "#8b5cf6",
  },
  {
    id: "modmenu",
    title: "Mod Menu",
    author: "Prospector",
    description: "A simple UI for managing mods in-game.",
    downloads: 12_600_000,
    follows: 2500,
    badge: "M",
    accent: "#3b82f6",
  },
];

export const MOCK_NEWS_SLIDES = [
  {
    id: "new-journey",
    title: "A NEW JOURNEY AWAITS",
    subtitle: "Same game. More ways to play.",
  },
];
