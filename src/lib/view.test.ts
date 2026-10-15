import { PerspectiveCamera, Vector3 } from "three";
import { describe, expect, it } from "vitest";
import {
  askView,
  cameraPos,
  clampView,
  DEFAULT_VIEW,
  fitDistance,
  flipYaw,
  FOV,
  PITCH_MAX,
  PITCH_MIN,
  viewFromCamera,
  wrapAngle,
  type ViewState,
} from "./view";

const PI = Math.PI;

describe("3D view helpers", () => {
  it("wraps yaw a full turn, clamps tilt above the horizon and zoom to 100-250%", () => {
    expect(clampView({ yaw: 0.3, pitch: 0.9, zoom: 1.2 })).toEqual({ yaw: 0.3, pitch: 0.9, zoom: 1.2 });
    expect(clampView({ yaw: 0, pitch: 0, zoom: 3 })).toEqual({ yaw: 0, pitch: PITCH_MIN, zoom: 2.5 });
    expect(clampView({ yaw: 0, pitch: PI / 2, zoom: 0.5 })).toEqual({ yaw: 0, pitch: PITCH_MAX, zoom: 1 });
    expect(clampView({ yaw: 2 * PI + 0.3, pitch: 1, zoom: 1 }).yaw).toBeCloseTo(0.3);
    expect(clampView({ yaw: -2 * PI - 0.3, pitch: 1, zoom: 1 }).yaw).toBeCloseTo(-0.3);
  });

  it("keeps in-range yaw untouched, including both ends", () => {
    expect(wrapAngle(PI)).toBe(PI);
    expect(wrapAngle(-PI)).toBe(-PI);
    expect(Math.abs(wrapAngle(3 * PI))).toBeCloseTo(PI);
  });

  it("flips to the other side and back", () => {
    expect(flipYaw(0)).toBe(PI);
    expect(flipYaw(PI)).toBeCloseTo(0);
    expect(flipYaw(-0.2)).toBeCloseTo(PI - 0.2);
    expect(flipYaw(0.4)).toBeCloseTo(0.4 - PI);
  });

  it("never sends the camera's own moves back to the camera", () => {
    const start: ViewState = { asked: DEFAULT_VIEW, shown: DEFAULT_VIEW };
    // a right-drag: the camera reports where it is now; the view it was last asked for stays the same object
    const dragged = viewFromCamera(start, { yaw: 0.5, pitch: 1.2, zoom: 1.4 });
    expect(dragged.asked).toBe(start.asked);
    expect(dragged.shown).toEqual({ yaw: 0.5, pitch: 1.2, zoom: 1.4 });
    // a slider asks for a view built on what is shown, as a new object the camera moves to once
    const slid = askView(dragged, { yaw: -0.3 });
    expect(slid.asked).not.toBe(dragged.asked);
    expect(slid.asked).toEqual({ yaw: -0.3, pitch: 1.2, zoom: 1.4 });
    expect(slid.shown).toEqual(slid.asked);
    // Reset view asks again even when the values equal the last request
    expect(askView(start, DEFAULT_VIEW).asked).not.toBe(start.asked);
    expect(askView(start, { zoom: 9 }).asked.zoom).toBe(2.5);
  });

  it("fits the whole plate on screen at 100% for every yaw and tilt", () => {
    const [hw, hd, top] = [8.3, 3.7, 0.51]; // roughly the real plate, keycap tops included
    const radius = Math.hypot(hw, hd, top);
    for (const aspect of [0.7, 1, 1.25, 1.8]) {
      const fit = fitDistance(radius, aspect);
      const cam = new PerspectiveCamera(FOV, aspect, 0.1, 1000);
      for (let deg = -180; deg < 180; deg += 5) {
        for (const pitch of [PITCH_MIN, 0.3, 0.6, DEFAULT_VIEW.pitch, 1.1, PITCH_MAX]) {
          cam.position.set(...cameraPos({ yaw: (deg * PI) / 180, pitch, zoom: 1 }, fit));
          cam.lookAt(0, 0, 0);
          cam.updateMatrixWorld();
          for (let c = 0; c < 8; c++) {
            const p = new Vector3(c & 1 ? hw : -hw, c & 2 ? top : -top, c & 4 ? hd : -hd).project(cam);
            expect(Math.max(Math.abs(p.x), Math.abs(p.y))).toBeLessThan(1);
          }
        }
      }
    }
  });
});
