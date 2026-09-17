import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { DirectConnect, LaunchEvent } from "@/types/launch";

const LAUNCH_EVENT = "launch://event";

/**
 * Resolves with the process's exit code once Minecraft is closed.
 * `directConnect` auto-connects to a specific server on launch (the LAN
 * "Join" flow) - omitted for a normal Play launch that just opens the menu.
 */
export function launchInstance(
  id: string,
  concurrency: number,
  directConnect?: DirectConnect,
): Promise<number> {
  return invoke<number>("launch_instance", { id, concurrency, directConnect });
}

export function listenToLaunchEvents(callback: (event: LaunchEvent) => void): Promise<UnlistenFn> {
  return listen<LaunchEvent>(LAUNCH_EVENT, (event) => callback(event.payload));
}
