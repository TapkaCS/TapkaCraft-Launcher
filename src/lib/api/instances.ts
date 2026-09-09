/**
 * Typed wrappers around the `*_instance(s)` Tauri commands
 * (`src-tauri/src/commands/instances.rs`). These always talk to the real
 * Rust backend - they don't know about `isTauri`/browser-preview mode.
 * `instanceStore` is the layer that decides whether to call these or fall
 * back to in-memory mock data.
 */
import { invoke } from "@tauri-apps/api/core";

import type { InstanceMeta, JavaInstanceSettings, LoaderConfig } from "@/types/instance";

export interface NewInstanceInput {
  name: string;
  minecraftVersion: string;
  loader: LoaderConfig;
}

/** Mirrors the Rust `InstanceUpdate` patch - only set fields are changed. */
export interface InstanceUpdateInput {
  name?: string;
  favorite?: boolean;
  java?: JavaInstanceSettings;
}

export function listInstances(): Promise<InstanceMeta[]> {
  return invoke<InstanceMeta[]>("list_instances");
}

export function createInstance(input: NewInstanceInput): Promise<InstanceMeta> {
  return invoke<InstanceMeta>("create_instance", { input });
}

export function updateInstance(id: string, patch: InstanceUpdateInput): Promise<InstanceMeta> {
  return invoke<InstanceMeta>("update_instance", { id, patch });
}

export function deleteInstance(id: string): Promise<void> {
  return invoke<void>("delete_instance", { id });
}

export function openInstanceFolder(id: string): Promise<void> {
  return invoke<void>("open_instance_folder", { id });
}
