import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { LaunchEvent } from "@/types/launch";

const LAUNCH_EVENT = "launch://event";

/** Resolves with the process's exit code once Minecraft is closed. */
export function launchInstance(id: string, concurrency: number): Promise<number> {
  return invoke<number>("launch_instance", { id, concurrency });
}

export function listenToLaunchEvents(callback: (event: LaunchEvent) => void): Promise<UnlistenFn> {
  return listen<LaunchEvent>(LAUNCH_EVENT, (event) => callback(event.payload));
}
