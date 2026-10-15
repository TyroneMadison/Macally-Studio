import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import Keyboard from "./Keyboard";
import layoutJson from "../lib/layout.json";
import type { AppState } from "../lib/state";
import type { Layout, Profile, Settings } from "../lib/types";

// WebGL is not under test: the stub only reports what the view passes in.
vi.mock("../components/Keyboard3D", () => ({
  default: (p: { boundCodes: Set<string>; legends: string }) => (
    <div data-bound={[...p.boundCodes].join(",")} data-legends={p.legends} />
  ),
}));

const mine: Profile = {
  schema: 1,
  id: "mine",
  name: "Mine",
  builtin: false,
  hotkey: null,
  autoApps: [],
  bindings: {
    F13: { mods: [], action: { type: "pass_through" } },
    F14: { mods: [], action: { type: "pause_toggle" } },
  },
};

const store = (activeProfile: string) =>
  ({
    layout: layoutJson as Layout,
    profiles: [mine],
    settings: { activeProfile, windowsLegends: true } as Settings,
    activeProfile: [mine].find((p) => p.id === activeProfile) ?? null,
    catalog: [],
    lastKey: null,
  }) as unknown as AppState;

describe("Keyboard view", () => {
  it("lights the active profile's keys and mounts the Inspector", () => {
    const html = renderToStaticMarkup(<Keyboard store={store("mine")} />);
    expect(html).toContain('data-bound="F13,F14" data-legends="win"');
    expect(html).toContain("Click a key on the keyboard to change it.");
    expect(html).not.toContain("Pick key by pressing");
    expect(html).toContain("Press any key to see what the OS reports");
    expect(html).toContain("Tip: press fn + esc so the top row sends F1-F15");
  });

  it("offers a full turn, tilt, zoom and lighting, defaulting to the product shot at full backlight", () => {
    const html = renderToStaticMarkup(<Keyboard store={store("mine")} />);
    expect(html).toMatch(/min="-180" max="180" step="1" aria-label="Rotate" value="-14"/);
    expect(html).toMatch(/min="10" max="85" step="1" aria-label="Tilt" value="38"/);
    expect(html).toMatch(/min="50" max="200" step="1" aria-label="Brightness" value="100"/);
    expect(html).toContain('<button type="button" aria-pressed="true">High</button>');
  });

  it("waits for profiles before showing the Inspector", () => {
    const html = renderToStaticMarkup(<Keyboard store={store("missing")} />);
    expect(html).toContain('data-bound=""');
    expect(html).toContain("Loading profiles…");
  });
});
