/** Camera state of the 3D keyboard, radians: yaw 0 = straight on and turns a full circle, pitch is the
 * tilt from straight down, zoom 1 = 100%. */
export type View3D = { yaw: number; pitch: number; zoom: number };

export const FOV = 32;
export const PITCH_MIN = (5 * Math.PI) / 180; // almost straight down
export const PITCH_MAX = (80 * Math.PI) / 180; // stays above the horizon, so the underside never shows
export const ZOOM_MIN = 1;
export const ZOOM_MAX = 2.5;
// 100% keeps the whole board in view at any angle; the default starts closer for the usual front view.
export const DEFAULT_VIEW: View3D = { yaw: (-14 * Math.PI) / 180, pitch: 0.9, zoom: 1.4 };

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

/** Wraps an angle into [-PI, PI]; in-range values pass through untouched so sliders do not jump. */
export const wrapAngle = (a: number) => (Math.abs(a) <= Math.PI ? a : a - 2 * Math.PI * Math.round(a / (2 * Math.PI)));

export function clampView(v: View3D): View3D {
  return {
    yaw: wrapAngle(v.yaw),
    pitch: clamp(v.pitch, PITCH_MIN, PITCH_MAX),
    zoom: clamp(v.zoom, ZOOM_MIN, ZOOM_MAX),
  };
}

/** "Flip 180°": look at the board from the other side. */
export const flipYaw = (yaw: number) => wrapAngle(yaw + Math.PI);

/** Camera distance at 100% zoom: R / sin(narrower half-FOV) keeps a sphere of `radius` (half the plate's
 * diagonal) inside the frustum, so no yaw or tilt can crop the board. 1.03 leaves a hair of air. */
export function fitDistance(radius: number, aspect: number) {
  const t = Math.tan((FOV / 2) * (Math.PI / 180)) * Math.min(aspect, 1);
  return ((radius * Math.hypot(1, t)) / t) * 1.03;
}

/** Camera position orbiting the origin (three.js Spherical: phi from +Y, theta from +Z towards +X). */
export function cameraPos({ yaw, pitch, zoom }: View3D, fit: number): [number, number, number] {
  const r = fit / zoom;
  return [r * Math.sin(pitch) * Math.sin(yaw), r * Math.cos(pitch), r * Math.sin(pitch) * Math.cos(yaw)];
}
