import { beforeEach, describe, expect, it } from "vitest";

import { MOCK_INSTANCES } from "@/lib/mock/mockData";

import { useInstanceStore } from "./instanceStore";

// vitest runs in jsdom, which has no `__TAURI_INTERNALS__` global, so
// `isTauri` is false throughout this file and every action below exercises
// the in-memory dev-mode fallback path - the same path a plain-browser UI
// preview uses. The real Tauri-backed path is covered by the Rust-side
// `core::instances::service` tests instead.
describe("instanceStore", () => {
  beforeEach(() => {
    useInstanceStore.setState({
      instances: [],
      selectedInstanceId: null,
      status: "idle",
      error: null,
    });
  });

  it("starts idle with no instances until loadInstances runs", () => {
    const state = useInstanceStore.getState();
    expect(state.status).toBe("idle");
    expect(state.instances).toEqual([]);
  });

  it("loadInstances seeds the mock fallback and selects the favorite", async () => {
    await useInstanceStore.getState().loadInstances();
    const state = useInstanceStore.getState();
    expect(state.status).toBe("ready");
    expect(state.instances).toEqual(MOCK_INSTANCES);
    expect(state.selectedInstanceId).toBe(MOCK_INSTANCES.find((i) => i.favorite)?.id);
  });

  it("creates a new instance with a unique slug id and selects it", async () => {
    await useInstanceStore.getState().loadInstances();
    const before = useInstanceStore.getState().instances.length;

    const created = await useInstanceStore.getState().createInstance({
      name: "My Test Profile",
      minecraftVersion: "1.21.1",
      loader: "fabric",
    });

    const state = useInstanceStore.getState();
    expect(state.instances).toHaveLength(before + 1);
    expect(state.selectedInstanceId).toBe(created.id);
    expect(created.id).toBe("my-test-profile");
    expect(created.loader).toEqual({ type: "fabric", version: "latest" });
  });

  it("uses the minecraft version as the vanilla loader version", async () => {
    const created = await useInstanceStore.getState().createInstance({
      name: "Vanilla Test",
      minecraftVersion: "1.20.1",
      loader: "vanilla",
    });
    expect(created.loader).toEqual({ type: "vanilla", version: "1.20.1" });
  });

  it("avoids id collisions when two profile names slugify the same", async () => {
    const first = await useInstanceStore
      .getState()
      .createInstance({ name: "Dup", minecraftVersion: "1.21.1", loader: "vanilla" });
    const second = await useInstanceStore
      .getState()
      .createInstance({ name: "Dup", minecraftVersion: "1.21.1", loader: "vanilla" });
    expect(first.id).not.toBe(second.id);
  });

  it("renames only the targeted instance", async () => {
    await useInstanceStore.getState().loadInstances();
    const targetId = MOCK_INSTANCES[0].id;

    await useInstanceStore.getState().renameInstance(targetId, "  New Name  ");

    const state = useInstanceStore.getState();
    expect(state.instances.find((i) => i.id === targetId)?.name).toBe("New Name");
    expect(state.instances.find((i) => i.id === MOCK_INSTANCES[1].id)?.name).toBe(
      MOCK_INSTANCES[1].name,
    );
  });

  it("toggles favorite state for only the targeted instance", async () => {
    await useInstanceStore.getState().loadInstances();
    const targetId = MOCK_INSTANCES[1].id;
    const otherId = MOCK_INSTANCES[2].id;

    await useInstanceStore.getState().toggleFavorite(targetId);
    let state = useInstanceStore.getState();
    expect(state.instances.find((i) => i.id === targetId)?.favorite).toBe(true);
    expect(state.instances.find((i) => i.id === otherId)?.favorite).toBe(false);

    await useInstanceStore.getState().toggleFavorite(targetId);
    state = useInstanceStore.getState();
    expect(state.instances.find((i) => i.id === targetId)?.favorite).toBe(false);
  });

  it("deletes an instance and re-picks a selection if it was selected", async () => {
    await useInstanceStore.getState().loadInstances();
    const favorite = MOCK_INSTANCES.find((i) => i.favorite);
    expect(favorite).toBeDefined();
    const targetId = favorite?.id ?? "";
    expect(useInstanceStore.getState().selectedInstanceId).toBe(targetId);

    await useInstanceStore.getState().deleteInstance(targetId);

    const state = useInstanceStore.getState();
    expect(state.instances.some((i) => i.id === targetId)).toBe(false);
    expect(state.selectedInstanceId).not.toBe(targetId);
    expect(state.selectedInstanceId).not.toBeNull();
  });

  it("rejects opening an instance folder outside the Tauri webview", async () => {
    await expect(useInstanceStore.getState().openInstanceFolder("anything")).rejects.toThrow();
  });
});
