/** Camera state of the 3D keyboard: yaw in radians (0 = straight on), zoom 1 = 100%. */
export type View3D = { yaw: number; zoom: number };

export const YAW_LIMIT = Math.PI / 2; // +-90 deg, a 180 deg sweep
export const ZOOM_MIN = 1;
export const ZOOM_MAX = 1.5;
export const DEFAULT_VIEW: View3D = { yaw: (-14 * Math.PI) / 180, zoom: 1 };

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

export function clampView(v: View3D): View3D {
  return { yaw: clamp(v.yaw, -YAW_LIMIT, YAW_LIMIT), zoom: clamp(v.zoom, ZOOM_MIN, ZOOM_MAX) };
}

/** "Flip 180°": jump to the opposite end of the sweep. */
export function flipYaw(yaw: number): number {
  return yaw <= 0 ? YAW_LIMIT : -YAW_LIMIT;
}
