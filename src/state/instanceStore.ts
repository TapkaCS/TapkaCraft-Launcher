import { create } from "zustand";

import { MOCK_INSTANCES } from "@/lib/mock/mockData";
import type { InstanceMeta, LoaderKind } from "@/types/instance";

export interface NewProfileInput {
  name: string;
  minecraftVersion: string;
  loader: LoaderKind;
}

interface InstanceStore {
  instances: InstanceMeta[];
  selectedInstanceId: string | null;
  selectInstance: (id: string) => void;

  /**
   * Appends a profile to in-memory state only - nothing is written to
   * disk. Real instance creation (directory scaffolding under
   * `instances/<id>/` and `instance.json` persistence via
   * `InstanceService`) is Phase 2; see `src-tauri/src/core/instances`.
   * Returns the created instance so the caller (the New Profile dialog)
   * can close itself and select it.
   */
  createInstance: (input: NewProfileInput) => InstanceMeta;
  toggleFavorite: (id: string) => void;
}

function slugify(name: string): string {
  const slug = name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/(^-|-$)/g, "");
  return slug.length > 0 ? slug : "profile";
}

function defaultLoaderVersion(loader: LoaderKind, minecraftVersion: string): string {
  return loader === "vanilla" ? minecraftVersion : "latest";
}

const initialSelection =
  MOCK_INSTANCES.find((instance) => instance.favorite)?.id ?? MOCK_INSTANCES[0]?.id ?? null;

export const useInstanceStore = create<InstanceStore>((set, get) => ({
  instances: MOCK_INSTANCES,
  selectedInstanceId: initialSelection,

  selectInstance: (id) => set({ selectedInstanceId: id }),

  createInstance: (input) => {
    const existingIds = new Set(get().instances.map((i) => i.id));
    let id = slugify(input.name);
    while (existingIds.has(id)) {
      id = `${id}-${Math.random().toString(36).slice(2, 6)}`;
    }

    const instance: InstanceMeta = {
      id,
      name: input.name,
      minecraftVersion: input.minecraftVersion,
      loader: {
        type: input.loader,
        version: defaultLoaderVersion(input.loader, input.minecraftVersion),
      },
      java: { memoryMinMb: 1024, memoryMaxMb: 4096 },
      favorite: false,
      playtimeSeconds: 0,
      launchCount: 0,
    };

    set({
      instances: [...get().instances, instance],
      selectedInstanceId: instance.id,
    });
    return instance;
  },

  toggleFavorite: (id) =>
    set({
      instances: get().instances.map((instance) =>
        instance.id === id ? { ...instance, favorite: !instance.favorite } : instance,
      ),
    }),
}));
