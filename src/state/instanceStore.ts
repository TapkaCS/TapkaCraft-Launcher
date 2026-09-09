import { create } from "zustand";

import {
  createInstance as createInstanceCommand,
  deleteInstance as deleteInstanceCommand,
  listInstances,
  openInstanceFolder as openInstanceFolderCommand,
  updateInstance as updateInstanceCommand,
} from "@/lib/api/instances";
import { MOCK_INSTANCES } from "@/lib/mock/mockData";
import { isTauri } from "@/lib/tauri";
import type { InstanceMeta, LoaderKind } from "@/types/instance";

export interface NewProfileInput {
  name: string;
  minecraftVersion: string;
  loader: LoaderKind;
}

type LoadStatus = "idle" | "loading" | "ready" | "error";

interface InstanceStore {
  instances: InstanceMeta[];
  selectedInstanceId: string | null;
  status: LoadStatus;
  error: string | null;

  /**
   * Populates `instances` from the real `InstanceService` over Tauri IPC
   * (`instances_dir()` on disk). Outside the Tauri webview - a plain
   * `vite dev` browser preview, with no Rust backend to call - this seeds
   * Phase 1's mock data instead. That fallback is for UI development only;
   * it is not a claim that persistence works without Tauri.
   */
  loadInstances: () => Promise<void>;
  selectInstance: (id: string) => void;
  createInstance: (input: NewProfileInput) => Promise<InstanceMeta>;
  renameInstance: (id: string, name: string) => Promise<void>;
  toggleFavorite: (id: string) => Promise<void>;
  deleteInstance: (id: string) => Promise<void>;
  openInstanceFolder: (id: string) => Promise<void>;
}

function defaultLoaderVersion(loader: LoaderKind, minecraftVersion: string): string {
  return loader === "vanilla" ? minecraftVersion : "latest";
}

function pickDefaultSelection(
  instances: InstanceMeta[],
  previousId?: string | null,
): string | null {
  if (previousId && instances.some((instance) => instance.id === previousId)) {
    return previousId;
  }
  return instances.find((instance) => instance.favorite)?.id ?? instances[0]?.id ?? null;
}

function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

// --- dev-mode fallback: id generation the real InstanceService would
// otherwise do on the Rust side (see core::instances::service::unique_id).
function mockUniqueId(existing: InstanceMeta[], name: string): string {
  const base = name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/(^-|-$)/g, "");
  const slug = base.length > 0 ? base : "profile";

  const existingIds = new Set(existing.map((instance) => instance.id));
  if (!existingIds.has(slug)) return slug;
  let n = 2;
  while (existingIds.has(`${slug}-${n}`)) n += 1;
  return `${slug}-${n}`;
}

export const useInstanceStore = create<InstanceStore>((set, get) => ({
  instances: [],
  selectedInstanceId: null,
  status: "idle",
  error: null,

  loadInstances: async () => {
    set({ status: "loading", error: null });
    try {
      const instances = isTauri ? await listInstances() : MOCK_INSTANCES;
      set({
        instances,
        status: "ready",
        selectedInstanceId: pickDefaultSelection(instances, get().selectedInstanceId),
      });
    } catch (err) {
      set({ status: "error", error: errorMessage(err) });
    }
  },

  selectInstance: (id) => set({ selectedInstanceId: id }),

  createInstance: async (input) => {
    const loader = {
      type: input.loader,
      version: defaultLoaderVersion(input.loader, input.minecraftVersion),
    };

    const created = isTauri
      ? await createInstanceCommand({
          name: input.name,
          minecraftVersion: input.minecraftVersion,
          loader,
        })
      : {
          id: mockUniqueId(get().instances, input.name),
          name: input.name.trim(),
          minecraftVersion: input.minecraftVersion,
          loader,
          java: { memoryMinMb: 1024, memoryMaxMb: 4096 },
          favorite: false,
          playtimeSeconds: 0,
          launchCount: 0,
        };

    set({ instances: [...get().instances, created], selectedInstanceId: created.id });
    return created;
  },

  renameInstance: async (id, name) => {
    if (!isTauri) {
      set({
        instances: get().instances.map((instance) =>
          instance.id === id ? { ...instance, name: name.trim() } : instance,
        ),
      });
      return;
    }
    const updated = await updateInstanceCommand(id, { name });
    set({
      instances: get().instances.map((instance) => (instance.id === id ? updated : instance)),
    });
  },

  toggleFavorite: async (id) => {
    const current = get().instances.find((instance) => instance.id === id);
    if (!current) return;

    if (!isTauri) {
      set({
        instances: get().instances.map((instance) =>
          instance.id === id ? { ...instance, favorite: !instance.favorite } : instance,
        ),
      });
      return;
    }
    const updated = await updateInstanceCommand(id, { favorite: !current.favorite });
    set({
      instances: get().instances.map((instance) => (instance.id === id ? updated : instance)),
    });
  },

  deleteInstance: async (id) => {
    if (isTauri) {
      await deleteInstanceCommand(id);
    }
    const remaining = get().instances.filter((instance) => instance.id !== id);
    const wasSelected = get().selectedInstanceId === id;
    set({
      instances: remaining,
      selectedInstanceId: wasSelected ? pickDefaultSelection(remaining) : get().selectedInstanceId,
    });
  },

  openInstanceFolder: async (id) => {
    if (!isTauri) {
      throw new Error("Opening instance folders isn't available in the browser preview.");
    }
    await openInstanceFolderCommand(id);
  },
}));
