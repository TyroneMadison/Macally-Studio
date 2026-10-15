import { describe, expect, it, vi } from "vitest";
import { showToast, subscribeToast } from "./toast";

describe("toast bus", () => {
  it("delivers a message to current subscribers only", () => {
    const seen: string[] = [];
    const off = subscribeToast((m) => seen.push(m));
    showToast("Fn reset across 3 profiles");
    expect(seen).toEqual(["Fn reset across 3 profiles"]);
    off();
    showToast("gone");
    expect(seen).toEqual(["Fn reset across 3 profiles"]);
  });

  it("does not throw with no subscribers", () => {
    expect(() => showToast("nobody listening")).not.toThrow();
  });

  it("isolates a throwing subscriber from the others", () => {
    const good = vi.fn();
    const offBad = subscribeToast(() => {
      throw new Error("boom");
    });
    const offGood = subscribeToast(good);
    showToast("x");
    expect(good).toHaveBeenCalledWith("x");
    offBad();
    offGood();
  });
});
