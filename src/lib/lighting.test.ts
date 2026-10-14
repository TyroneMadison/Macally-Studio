import { afterEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_LIGHTING, loadLighting, saveLighting } from "./lighting";

const memory = () => {
  const m = new Map<string, string>();
  return { getItem: (k: string) => m.get(k) ?? null, setItem: (k: string, v: string) => void m.set(k, v) };
};

describe("lighting storage", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("defaults to 100% and High, then remembers the viewer's choice", () => {
    vi.stubGlobal("localStorage", memory());
    expect(loadLighting()).toEqual(DEFAULT_LIGHTING);
    saveLighting({ brightness: 1.5, backlight: 0 });
    expect(loadLighting()).toEqual({ brightness: 1.5, backlight: 0 });
  });

  it("ignores out-of-range or corrupt values", () => {
    const s = memory();
    vi.stubGlobal("localStorage", s);
    s.setItem("macally-studio.lighting", '{"brightness":9,"backlight":1.5}');
    expect(loadLighting()).toEqual(DEFAULT_LIGHTING);
    s.setItem("macally-studio.lighting", "not json");
    expect(loadLighting()).toEqual(DEFAULT_LIGHTING);
  });

  it("survives blocked storage", () => {
    const blocked = () => {
      throw new Error("SecurityError");
    };
    vi.stubGlobal("localStorage", { getItem: blocked, setItem: blocked });
    expect(loadLighting()).toEqual(DEFAULT_LIGHTING);
    expect(() => saveLighting(DEFAULT_LIGHTING)).not.toThrow();
  });
});
