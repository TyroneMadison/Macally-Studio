import { describe, expect, it } from "vitest";
import { displayVersion } from "./version";

describe("displayVersion", () => {
  it("drops the semver patch", () => {
    expect(displayVersion("1.0.0")).toBe("1.0");
    expect(displayVersion("2.7.0")).toBe("2.7");
  });
  it("accepts a v prefix and a bare X.Y", () => {
    expect(displayVersion("v1.3.0")).toBe("1.3");
    expect(displayVersion("1.3")).toBe("1.3");
  });
  it("returns unparseable input unchanged", () => {
    expect(displayVersion("dev")).toBe("dev");
  });
});
