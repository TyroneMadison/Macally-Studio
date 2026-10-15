import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import GameEditor from "./GameEditor";
import KeyGrid from "../components/KeyGrid";
import layoutJson from "../lib/layout.json";
import { DEFAULT_ID } from "../lib/games";
import type { AppState } from "../lib/state";
import type { Game, Layout, Profile, Settings } from "../lib/types";

const layout = layoutJson as Layout;
const warframe: Game = { name: "Warframe", exe: "Warframe.x64.exe", autoSwitch: true };
const desktop: Profile = {
  schema: 1,
  id: DEFAULT_ID,
  name: "Windows Default",
  builtin: true,
  hotkey: null,
  autoApps: [],
  bindings: { F3: { mods: [], action: { type: "key_combo", keys: ["Meta", "Tab"] } } },
};
const wf: Profile = {
  schema: 1,
  id: "warframe",
  name: "Warframe",
  builtin: false,
  hotkey: null,
  autoApps: [],
  bindings: {
    Fn: { mods: [], action: { type: "key_combo", keys: ["ControlLeft"] } },
    MetaLeft: { mods: [], action: { type: "block" } },
    KeyQ: { mods: [], action: { type: "key_combo", keys: ["KeyC"] } },
  },
  game: warframe,
};

const store = (active: string) =>
  ({
    layout,
    profiles: [desktop, wf],
    settings: { activeProfile: active, gameAutoSwitch: true, windowsLegends: true } as Settings,
    activeProfile: [desktop, wf].find((p) => p.id === active) ?? null,
    catalog: [],
    lastKey: null,
    liveGame: null,
    switched: null,
    saveBinding: async () => {},
  }) as unknown as AppState;

const noop = () => {};

describe("KeyGrid", () => {
  it("draws every key, marking remapped and turned-off ones with what they do", () => {
    const html = renderToStaticMarkup(
      <KeyGrid layout={layout} bindings={wf.bindings} legends="win" selected="Fn" onSelect={noop} note={(b) => (b.action.type === "block" ? "Off" : "Ctrl")} />,
    );
    expect(html.match(/class="kg-key/g)).toHaveLength(layout.keys.length);
    expect(html).toMatch(/class="kg-key remapped selected"[^>]*data-code="Fn"/);
    expect(html).toMatch(/class="kg-key off"[^>]*data-code="MetaLeft"/);
    expect(html).toContain('<span class="kg-note">Off</span>');
    expect(html).toMatch(/class="kg-key"[^>]*data-code="KeyA"/);
  });

  it("shortens long Mac legends to fit a flat cap", () => {
    const html = renderToStaticMarkup(<KeyGrid layout={layout} bindings={{}} legends="mac" selected={null} onSelect={noop} note={() => ""} />);
    expect(html).toMatch(/data-code="MetaLeft"[^>]*><span class="kg-leg">cmd<\/span>/);
    expect(html).toMatch(/data-code="AltLeft"[^>]*><span class="kg-leg">opt<\/span>/);
  });
});

describe("Full-screen profile editor", () => {
  it("opens on the game's own keys, active and switching automatically", () => {
    const html = renderToStaticMarkup(<GameEditor store={store("warframe")} profileId="warframe" auto onBack={noop} onProfiles={noop} onExit={noop} />);
    expect(html).toContain("← Back");
    expect(html).toContain('<span class="ge-name">Warframe</span>');
    expect(html).toContain("Warframe.x64.exe");
    expect(html).toContain("Active for Warframe");
    expect(html).toMatch(/Auto-switch<input type="checkbox" role="switch" class="switch" aria-label="Switch to Warframe automatically" checked=""/);
    expect(html).toContain(">Profiles</button>");
    expect(html).toContain("✕ Exit");
    expect(html).toContain("Click any key to change it · Warframe only");
    expect(html).toContain("Remapped");
    expect(html).toContain("Disabled");
    expect(html).toContain("Opened automatically when Warframe started. Unchanged keys use your Default profile.");
    expect(html).toContain("Remaps in this profile · 3");
    expect(html).toContain("Click a key on the keyboard to change it.");
  });

  it("says when it was opened by hand, and is not active then", () => {
    const html = renderToStaticMarkup(<GameEditor store={store(DEFAULT_ID)} profileId="warframe" auto={false} onBack={noop} onProfiles={noop} onExit={noop} />);
    expect(html).toContain("Opened when you switched to Warframe. Unchanged keys use your Default profile.");
    expect(html).not.toContain("Active for Warframe");
  });

  it("shows a key's binding from the Default profile when the game leaves it alone", () => {
    const html = renderToStaticMarkup(
      <GameEditor store={store("warframe")} profileId="warframe" auto initialKey="F3" onBack={noop} onProfiles={noop} onExit={noop} />,
    );
    expect(html).toContain("From your Default profile");
    expect(html).toContain('<span class="insp-title">Win + Tab</span>');
  });

  it("edits the Default profile the same way, without game parts", () => {
    const html = renderToStaticMarkup(<GameEditor store={store(DEFAULT_ID)} profileId={DEFAULT_ID} auto={false} onBack={noop} onProfiles={noop} onExit={noop} />);
    expect(html).toContain('<span class="ge-name">Windows Default</span>');
    expect(html).toContain("Click any key to change it");
    expect(html).not.toContain(" only<");
    expect(html).not.toContain("Auto-switch");
    expect(html).not.toContain("Unchanged keys use your Default profile");
    expect(html).toContain("Remaps in this profile · 1");
  });
});
