import { afterEach, describe, expect, it, vi } from "vitest";
import { beginRecord } from "./record";
import type { KeyEvent } from "./types";

const ipc = vi.hoisted(() => ({
  listeners: new Set<(e: KeyEvent) => void>(),
  startRecord: vi.fn(() => Promise.resolve()),
  stopRecord: vi.fn(() => Promise.resolve()),
}));
vi.mock("./ipc", () => ({
  onKeyEvent: (cb: (e: KeyEvent) => void) => {
    ipc.listeners.add(cb);
    return Promise.resolve(() => ipc.listeners.delete(cb));
  },
  startRecord: ipc.startRecord,
  stopRecord: ipc.stopRecord,
}));

const press = (code: string) => ipc.listeners.forEach((l) => l({ code, down: true, mods: [], raw: "" }));
const settle = () => new Promise((r) => setTimeout(r));
/** A recording that takes the first key-down, the way the Keyboard view's picker does. */
const recorder = (got: string[]) => () => (e: KeyEvent) => (got.push(e.code), true);

describe("beginRecord", () => {
  afterEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it("ends on a finished key, and its cleanup leaves Record mode", async () => {
    const got: string[] = [];
    const end = vi.fn();
    const cleanup = beginRecord(recorder(got), end);
    await settle();
    expect(ipc.startRecord).toHaveBeenCalledOnce();
    press("KeyB");
    press("KeyC");
    expect(got).toEqual(["KeyB"]);
    expect(end).toHaveBeenCalledOnce();
    cleanup(); // the component turning `active` off after `end`
    await settle();
    expect(ipc.stopRecord).toHaveBeenCalledOnce();
    expect(ipc.listeners.size).toBe(0);
  });

  it("lets a second recording end the first and keep Record mode for itself", async () => {
    const [gotA, gotB]: string[][] = [[], []];
    const [endA, endB] = [vi.fn(), vi.fn()];
    const cleanupA = beginRecord(recorder(gotA), endA);
    await settle();
    const cleanupB = beginRecord(recorder(gotB), endB);
    expect(endA).toHaveBeenCalledOnce();
    cleanupA();
    await settle();
    expect(ipc.stopRecord).not.toHaveBeenCalled(); // only the owner leaves Record mode
    press("KeyB");
    expect(gotA).toEqual([]);
    expect(gotB).toEqual(["KeyB"]);
    cleanupB();
    expect(ipc.stopRecord).toHaveBeenCalledOnce();
  });

  it("ends on the engine's 15 s timeout", () => {
    vi.useFakeTimers();
    const end = vi.fn();
    const cleanup = beginRecord(recorder([]), end);
    vi.advanceTimersByTime(14_999);
    expect(end).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(end).toHaveBeenCalledOnce();
    cleanup();
  });
});
