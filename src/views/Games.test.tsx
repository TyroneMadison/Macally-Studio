import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import Games from "./Games";
import AddGame from "../components/AddGame";
import { DEFAULT_ID } from "../lib/games";
import type { AppState } from "../lib/state";
import type { AppEntry, Game, Profile, Settings } from "../lib/types";

const valorant: Game = { name: "Valorant", exe: "VALORANT.exe", path: "C:\\Riot Games\\VALORANT\\live\\VALORANT.exe", autoSwitch: true };
const portal: Game = { name: "Portal 2", exe: "", folder: "G:\\SteamLibrary\\steamapps\\common\\Portal 2", autoSwitch: false };

const profile = (id: string, name: string, bindings: Profile["bindings"], game?: Game): Profile => ({
  schema: 1,
  id,
  name,
  builtin: id === DEFAULT_ID,
  hotkey: null,
  autoApps: [],
  bindings,
  ...(game ? { game } : {}),
});

const desktop = profile(DEFAULT_ID, "Windows Default", {
  F3: { mods: [], action: { type: "key_combo", keys: ["Meta", "Tab"] } },
  F4: { mods: [], action: { type: "key_combo", keys: ["Meta"] } },
});
const val = profile(
  "valorant",
  "Valorant",
  {
    KeyQ: { mods: [], action: { type: "key_combo", keys: ["KeyC"] } },
    MetaLeft: { mods: [], action: { type: "block" } },
  },
  valorant,
);
const p2 = profile("portal-2", "Portal 2", {}, portal);

const store = (profiles: Profile[], active: string, gameAutoSwitch = true, liveGame: string | null = null) =>
  ({
    profiles,
    settings: { activeProfile: active, gameAutoSwitch, windowsLegends: true } as Settings,
    activeProfile: profiles.find((p) => p.id === active) ?? null,
    catalog: [],
    liveGame,
    switched: null,
  }) as unknown as AppState;

describe("Game profiles panel", () => {
  it("lists the Default and every game with its keys, exe, LIVE and switch", () => {
    const html = renderToStaticMarkup(<Games store={store([desktop, profile("work", "Work", {}), val, p2], "valorant", true, "valorant")} onEdit={() => {}} />);
    expect(html).toContain("Game profiles");
    expect(html).toContain("Each game keeps its own key bindings");
    expect(html).toContain("Game profiles · 2");
    expect(html).toContain("+ Add game");
    expect(html).toContain("Auto-detect is on");
    expect(html).toContain("Default (desktop)");
    expect(html).toContain("2 bindings · used when no game runs");
    expect(html).toContain("2 bindings · VALORANT.exe");
    expect(html).toContain('class="live">LIVE</span>');
    expect(html).toContain("0 bindings · Portal 2");
    expect(html).toContain('aria-label="Switch to Valorant automatically" checked=""');
    expect(html).not.toContain('aria-label="Switch to Portal 2 automatically" checked=""');
    expect(html).not.toContain(">Work<");
  });

  it("shows the selected game: where it lives, its switch and its own keys", () => {
    const html = renderToStaticMarkup(<Games store={store([desktop, val, p2], "valorant")} onEdit={() => {}} />);
    expect(html).toContain("<h2>Valorant</h2>");
    expect(html).toContain("C:\\Riot Games\\VALORANT\\live\\VALORANT.exe");
    expect(html).toContain(">Duplicate</button>");
    expect(html).toContain(">Delete</button>");
    expect(html).toContain("Switch automatically");
    expect(html).toContain("Apply these bindings when VALORANT.exe is in focus; go back to Default when it closes.");
    expect(html).toContain("Key bindings for this game · 2");
    expect(html).toContain(">Copy from Default</button>");
    expect(html).toContain(">+ Add binding</button>");
    expect(html).toContain('data-code="KeyQ"');
    expect(html).toContain(">Disabled in game<");
    expect(html).toContain(">Block<");
    expect(html).toContain("Keys not listed here fall back to your Default profile.");
  });

  it("says when detection is off", () => {
    const html = renderToStaticMarkup(<Games store={store([desktop, val], DEFAULT_ID, false)} onEdit={() => {}} />);
    expect(html).toContain("Auto-detect is off");
    expect(html).toContain('aria-label="Detect games and switch automatically"');
  });

  it("starts on the Default profile while there is no game yet", () => {
    const html = renderToStaticMarkup(<Games store={store([desktop], DEFAULT_ID)} onEdit={() => {}} />);
    expect(html).toContain("Game profiles · 0");
    expect(html).toContain("No games yet");
    expect(html).toContain("<h2>Default (desktop)</h2>");
    expect(html).toContain("Used when no game runs. Games use these keys unless they change them.");
    expect(html).toContain(">Edit keys</button>");
    expect(html).not.toContain("Switch automatically");
  });
});

const running: AppEntry[] = [
  { id: "running:C:\\Games\\Cyberpunk 2077\\bin\\x64\\Cyberpunk2077.exe", name: "Cyberpunk 2077", kind: "path", launch: "C:\\Games\\Cyberpunk 2077\\bin\\x64\\Cyberpunk2077.exe", icon: null, source: "running" },
  { id: "running:C:\\Program Files (x86)\\Steam\\steam.exe", name: "Steam", kind: "path", launch: "C:\\Program Files (x86)\\Steam\\steam.exe", icon: null, source: "running" },
];
const catalog: AppEntry[] = [
  { id: "steam:730", name: "Counter-Strike 2", kind: "steam", launch: "steam://rungameid/730", icon: null, source: "steam" },
  { id: "app:C:\\Tools\\tool.exe", name: "Tool", kind: "path", launch: "C:\\Tools\\tool.exe", icon: null, source: "windows" },
];

describe("Add a game", () => {
  it("offers what runs now and the installed games, with their exe", () => {
    const html = renderToStaticMarkup(<AddGame catalog={catalog} profiles={[desktop, profile("work", "Work", {}), val]} running={running} onAdd={async () => {}} onClose={() => {}} />);
    expect(html).toContain("Add a game");
    expect(html).toContain("Pick a running process or an installed program. Its profile switches on automatically when it starts.");
    expect(html).toContain("Running · 2");
    expect(html).toContain("Installed · 2");
    expect(html).toContain("Browse .exe…");
    expect(html).toContain('placeholder="Search programs and processes"');
    expect(html).toContain("Running now · 2");
    expect(html).toContain(">Cyberpunk2077.exe<");
    expect(html).toContain("Installed games · 1");
    expect(html).toContain(">Counter-Strike 2<");
    expect(html).not.toContain(">Tool<");
    expect(html).toMatch(/<option value="" selected="">Default profile<\/option><option value="work">Work<\/option><\/select>/);
    expect(html).toContain('class="btn primary" disabled="">Add game</button>');
  });

  it("waits for the running apps", () => {
    const html = renderToStaticMarkup(<AddGame catalog={catalog} profiles={[desktop]} running={null} onAdd={async () => {}} onClose={() => {}} />);
    expect(html).toContain("Looking for running programs…");
  });
});
