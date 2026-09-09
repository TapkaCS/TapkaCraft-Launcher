import { beforeEach, describe, expect, it } from "vitest";

import { MOCK_INSTANCES } from "@/lib/mock/mockData";

import { useInstanceStore } from "./instanceStore";

describe("instanceStore", () => {
  beforeEach(() => {
    useInstanceStore.setState({
      instances: MOCK_INSTANCES,
      selectedInstanceId: MOCK_INSTANCES[0]?.id ?? null,
    });
  });

  it("creates a new instance with a unique slug id and selects it", () => {
    const before = useInstanceStore.getState().instances.length;
    const created = useInstanceStore.getState().createInstance({
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

  it("uses the minecraft version as the vanilla loader version", () => {
    const created = useInstanceStore.getState().createInstance({
      name: "Vanilla Test",
      minecraftVersion: "1.20.1",
      loader: "vanilla",
    });
    expect(created.loader).toEqual({ type: "vanilla", version: "1.20.1" });
  });

  it("avoids id collisions when two profile names slugify the same", () => {
    const first = useInstanceStore
      .getState()
      .createInstance({ name: "Dup", minecraftVersion: "1.21.1", loader: "vanilla" });
    const second = useInstanceStore
      .getState()
      .createInstance({ name: "Dup", minecraftVersion: "1.21.1", loader: "vanilla" });
    expect(first.id).not.toBe(second.id);
  });

  it("toggles favorite state for only the targeted instance", () => {
    const targetId = MOCK_INSTANCES[1].id;
    const otherId = MOCK_INSTANCES[2].id;

    useInstanceStore.getState().toggleFavorite(targetId);
    let state = useInstanceStore.getState();
    expect(state.instances.find((i) => i.id === targetId)?.favorite).toBe(true);
    expect(state.instances.find((i) => i.id === otherId)?.favorite).toBe(false);

    useInstanceStore.getState().toggleFavorite(targetId);
    state = useInstanceStore.getState();
    expect(state.instances.find((i) => i.id === targetId)?.favorite).toBe(false);
  });
});
