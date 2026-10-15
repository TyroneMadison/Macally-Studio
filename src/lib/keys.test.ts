import { describe, expect, it } from "vitest";
import { isFirmwareOnly } from "./keys";

describe("firmware-only keys", () => {
  it("knows Fn is handled in firmware and nothing else is", () => {
    expect(isFirmwareOnly("Fn")).toBe(true);
    expect(isFirmwareOnly("KeyA")).toBe(false);
    expect(isFirmwareOnly("ControlLeft")).toBe(false);
    expect(isFirmwareOnly("F5")).toBe(false);
  });
});
