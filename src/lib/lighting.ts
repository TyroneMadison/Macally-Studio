/** Per-viewer 3D lighting: brightness scales every scene light (0.5-2), backlight follows the real
 * board's levels, 0 = off .. 3 = high. Kept in localStorage, which may be missing or blocked. */
export type Lighting = { brightness: number; backlight: number };

export const BACKLIGHT_LEVELS = ["Off", "Low", "Med", "High"] as const;
export const DEFAULT_LIGHTING: Lighting = { brightness: 1, backlight: 3 };
const KEY = "macally-studio.lighting";

export function loadLighting(): Lighting {
  try {
    const { brightness: b, backlight: l } = JSON.parse(localStorage.getItem(KEY) ?? "{}") as Record<string, unknown>;
    return {
      brightness: typeof b === "number" && b >= 0.5 && b <= 2 ? b : DEFAULT_LIGHTING.brightness,
      backlight: typeof l === "number" && [0, 1, 2, 3].includes(l) ? l : DEFAULT_LIGHTING.backlight,
    };
  } catch {
    return DEFAULT_LIGHTING;
  }
}

export function saveLighting(l: Lighting) {
  try {
    localStorage.setItem(KEY, JSON.stringify(l));
  } catch {
    // storage blocked: the choice lasts for this session only
  }
}
