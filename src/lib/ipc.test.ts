import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(async () => null) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import * as ipc from "./ipc";

describe("ipc", () => {
  beforeEach(() => vi.clearAllMocks());

  it("calls snake_case commands with named args", async () => {
    await ipc.exportProfile("photo", "C:/out.json");
    expect(invoke).toHaveBeenCalledWith("export_profile", { id: "photo", path: "C:/out.json" });
    await ipc.resetProfile("windows-default");
    expect(invoke).toHaveBeenCalledWith("reset_profile", { id: "windows-default" });
    expect("setPaused" in ipc || "onPausedChanged" in ipc).toBe(false); // pausing is gone
    await ipc.getStatus();
    expect(invoke).toHaveBeenCalledWith("get_status");
  });

  it("turns the Key Monitor on and off, and has no key recording", async () => {
    await ipc.setMonitor(true);
    await ipc.setMonitor(false);
    expect(vi.mocked(invoke).mock.calls).toEqual([
      ["set_monitor", { on: true }],
      ["set_monitor", { on: false }],
    ]);
    expect("startRecord" in ipc || "stopRecord" in ipc).toBe(false);
  });

  it("hands listeners the event payload", async () => {
    const cb = vi.fn();
    await ipc.onProfileChanged(cb);
    expect(listen).toHaveBeenCalledWith("profile-changed", expect.any(Function));
    const handler = vi.mocked(listen).mock.calls[0][1];
    handler({ event: "profile-changed", id: 1, payload: { id: "gaming" } });
    expect(cb).toHaveBeenCalledWith({ id: "gaming" });
  });
});
