import { beforeEach, describe, expect, it } from "vitest";

import type { InstallProgressEvent } from "@/types/version";

import { applyInstallProgress, useInstallStore } from "./installStore";

// vitest runs in jsdom, which has no `__TAURI_INTERNALS__` global, so
// `isTauri` is false throughout this file - `install()` always takes its
// "desktop app required" branch here. The real Tauri-backed install
// pipeline is covered by the Rust-side `core::versions::install` and
// `commands::versions` code instead.
describe("installStore", () => {
  beforeEach(() => {
    useInstallStore.setState({
      installingInstanceId: null,
      phase: "idle",
      progress: null,
      error: null,
      installedThisSession: new Set(),
    });
  });

  describe("applyInstallProgress (pure reducer)", () => {
    it("started resets progress and records the file count", () => {
      const event: InstallProgressEvent = { type: "started", totalFiles: 12 };
      const result = applyInstallProgress(null, event);
      expect(result).toEqual({
        totalFiles: 12,
        completedFiles: 0,
        failedFiles: 0,
        currentLabel: null,
      });
    });

    it("fileProgress updates the current label without touching counts", () => {
      const base = { totalFiles: 5, completedFiles: 2, failedFiles: 0, currentLabel: null };
      const result = applyInstallProgress(base, {
        type: "fileProgress",
        label: "client.jar",
        bytesDownloaded: 100,
        totalBytes: 500,
      });
      expect(result).toEqual({ ...base, currentLabel: "client.jar" });
    });

    it("fileCompleted increments completedFiles", () => {
      const base = { totalFiles: 5, completedFiles: 2, failedFiles: 0, currentLabel: null };
      const result = applyInstallProgress(base, {
        type: "fileCompleted",
        label: "brigadier.jar",
        cached: false,
      });
      expect(result.completedFiles).toBe(3);
      expect(result.currentLabel).toBe("brigadier.jar");
    });

    it("fileFailed increments failedFiles without touching completedFiles", () => {
      const base = { totalFiles: 5, completedFiles: 2, failedFiles: 0, currentLabel: null };
      const result = applyInstallProgress(base, {
        type: "fileFailed",
        label: "lwjgl.jar",
        error: "hash mismatch",
      });
      expect(result.failedFiles).toBe(1);
      expect(result.completedFiles).toBe(2);
    });

    it("finished clears the current label but keeps the tallies", () => {
      const base = { totalFiles: 5, completedFiles: 5, failedFiles: 0, currentLabel: "asset.ogg" };
      const result = applyInstallProgress(base, { type: "finished", succeeded: 5, failed: 0 });
      expect(result).toEqual({ ...base, currentLabel: null });
    });

    it("treats a null starting point the same as empty progress", () => {
      const result = applyInstallProgress(null, {
        type: "fileCompleted",
        label: "x.jar",
        cached: true,
      });
      expect(result.completedFiles).toBe(1);
      expect(result.totalFiles).toBe(0);
    });
  });

  describe("install (browser-preview fallback)", () => {
    it("reports an error instead of throwing when Tauri is unavailable", async () => {
      await expect(useInstallStore.getState().install("some-id")).resolves.toBeUndefined();
      const state = useInstallStore.getState();
      expect(state.phase).toBe("error");
      expect(state.error).toMatch(/desktop app/i);
      expect(state.installingInstanceId).toBeNull();
    });

    it("does not start a second install while one is already running", async () => {
      useInstallStore.setState({ installingInstanceId: "already-running", phase: "installing" });

      await useInstallStore.getState().install("another-id");

      const state = useInstallStore.getState();
      expect(state.installingInstanceId).toBe("already-running");
      expect(state.phase).toBe("installing");
      expect(state.error).toBeNull();
    });
  });
});
