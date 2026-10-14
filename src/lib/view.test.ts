import { describe, expect, it } from "vitest";
import { clampView, flipYaw, YAW_LIMIT } from "./view";

describe("3D view helpers", () => {
  it("clamps yaw to +-90 deg and zoom to 100-150%", () => {
    expect(clampView({ yaw: 3, zoom: 2 })).toEqual({ yaw: YAW_LIMIT, zoom: 1.5 });
    expect(clampView({ yaw: -3, zoom: 0.5 })).toEqual({ yaw: -YAW_LIMIT, zoom: 1 });
    expect(clampView({ yaw: 0.3, zoom: 1.2 })).toEqual({ yaw: 0.3, zoom: 1.2 });
  });

  it("flips to the opposite end of the sweep", () => {
    expect(flipYaw(-YAW_LIMIT)).toBe(YAW_LIMIT);
    expect(flipYaw(-0.2)).toBe(YAW_LIMIT);
    expect(flipYaw(0.4)).toBe(-YAW_LIMIT);
  });
});
