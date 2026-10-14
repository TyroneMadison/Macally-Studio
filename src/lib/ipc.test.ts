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
    await ipc.setPaused(true);
    expect(invoke).toHaveBeenCalledWith("set_paused", { paused: true });
    await ipc.getStatus();
    expect(invoke).toHaveBeenCalledWith("get_status");
  });

  it("ends a Record in Monitor mode only while Monitor is wanted", async () => {
    const modes = () =>
      vi.mocked(invoke).mock.calls.map((c) => (c[0] === "set_monitor" ? `monitor:${(c[1] as { on: boolean }).on}` : c[0]));
    await ipc.setMonitor(true);
    await ipc.startRecord();
    await ipc.setMonitor(true); // focus regained mid-Record: must not cancel the Record
    await ipc.stopRecord();
    await ipc.setMonitor(false);
    await ipc.startRecord();
    await ipc.stopRecord();
    expect(modes()).toEqual([
      "monitor:true",
      "start_record",
      "stop_record",
      "monitor:true",
      "monitor:false",
      "start_record",
      "stop_record",
    ]);
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
