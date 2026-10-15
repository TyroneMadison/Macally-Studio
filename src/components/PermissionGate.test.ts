import { describe, expect, it } from "vitest";
import { needsPermissions } from "./PermissionGate";
import type { Status } from "../lib/types";

const status = (platform: Status["platform"], permissionsOk: boolean): Status => ({
  activeProfile: "macos-default",
  hookRunning: permissionsOk,
  deviceFound: true,
  version: "1.0.0",
  platform,
  permissionsOk,
});

describe("needsPermissions", () => {
  it("blocks only on macOS while a permission is missing", () => {
    expect(needsPermissions(status("macos", false))).toBe(true);
    expect(needsPermissions(status("macos", true))).toBe(false);
  });

  it("never blocks on Windows or before the status has loaded", () => {
    expect(needsPermissions(status("windows", false))).toBe(false);
    expect(needsPermissions(null)).toBe(false);
  });
});
