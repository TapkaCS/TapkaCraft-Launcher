/**
 * Typed wrappers around the LAN Discovery Tauri commands
 * (`src-tauri/src/commands/lan.rs`).
 */
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { LanEvent } from "@/types/lan";

const LAN_EVENT = "lan://event";

export function startLanDiscovery(): Promise<void> {
  return invoke<void>("start_lan_discovery");
}

export function stopLanDiscovery(): Promise<void> {
  return invoke<void>("stop_lan_discovery");
}

export function listenToLanEvents(callback: (event: LanEvent) => void): Promise<UnlistenFn> {
  return listen<LanEvent>(LAN_EVENT, (event) => callback(event.payload));
}
