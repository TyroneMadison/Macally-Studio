import { describe, expect, it } from "vitest";
import layoutJson from "./layout.json";
import { KEY_CODES } from "./keycodes";
import type { Layout, LayoutKey } from "./types";

const layout: Layout = layoutJson;
const keys = layout.keys;
const byCode = (code: string): LayoutKey => keys.find((k) => k.code === code)!;
// row 0 (F row) is 0.6u tall, rows 1-5 are 1u and start right under it
const top = (k: LayoutKey) => (k.row === 0 ? 0 : 0.6 + (k.row - 1));

describe("layout.json", () => {
  it("has 107 keys with unique codes", () => {
    expect(keys).toHaveLength(107);
    expect(new Set(keys.map((k) => k.code)).size).toBe(107);
  });

  it("only uses codes the keymap knows", () => {
    const known = new Set(KEY_CODES);
    expect(keys.filter((k) => !known.has(k.code)).map((k) => k.code)).toEqual([]);
  });

  it("has no overlapping keys", () => {
    // ponytail: O(n^2) pairwise scan, 107 keys = ~5.7k checks
    const EPS = 1e-6;
    const hits: string[] = [];
    for (let i = 0; i < keys.length; i++) {
      for (let j = i + 1; j < keys.length; j++) {
        const a = keys[i], b = keys[j];
        const apart =
          a.x + a.w <= b.x + EPS || b.x + b.w <= a.x + EPS ||
          top(a) + a.h <= top(b) + EPS || top(b) + b.h <= top(a) + EPS;
        if (!apart) hits.push(`${a.code} x ${b.code}`);
      }
    }
    expect(hits).toEqual([]);
  });

  it("matches the Macally geometry", () => {
    expect(keys.filter((k) => k.row === 0)).toHaveLength(17);
    expect(keys.filter((k) => k.row === 0).every((k) => k.h === 0.6)).toBe(true);
    expect(byCode("Eject").x + byCode("Eject").w).toBeCloseTo(15);
    expect(byCode("Insert").x).toBe(15.5);
    expect(byCode("NumLock").x).toBe(19);
    expect(byCode("NumpadEnter")).toMatchObject({ row: 4, x: 22, h: 2 });
    expect(byCode("Numpad0")).toMatchObject({ row: 5, w: 2 });
    for (const row of [1, 2, 3, 4, 5]) {
      const main = keys.filter((k) => k.row === row && k.x < 15);
      expect(Math.max(...main.map((k) => k.x + k.w))).toBeCloseTo(15);
    }
  });
});
