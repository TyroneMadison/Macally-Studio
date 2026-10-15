import { describe, expect, it } from "vitest";
import {
  bindingCount,
  capNote,
  chipOf,
  entryExe,
  copyFromDefault,
  DEFAULT_ID,
  defaultProfile,
  gameExe,
  gamePlace,
  games,
  switchedIn,
  switchedAuto,
  switchesAuto,
  tileColors,
} from "./games";
import type { Game, Profile, Settings } from "./types";

const valorant: Game = { name: "Valorant", exe: "VALORANT.exe", path: "C:\\Riot Games\\VALORANT\\live\\VALORANT.exe", autoSwitch: true };
const portal: Game = { name: "Portal 2", exe: "", folder: "G:\\SteamLibrary\\steamapps\\common\\Portal 2", autoSwitch: true };

const profile = (id: string, game?: Game, bindings: Profile["bindings"] = {}): Profile => ({
  schema: 1,
  id,
  name: game?.name ?? id,
  builtin: id === DEFAULT_ID,
  hotkey: null,
  autoApps: [],
  bindings,
  ...(game ? { game } : {}),
});

const list = [
  profile(DEFAULT_ID, undefined, { F3: { mods: [], action: { type: "key_combo", keys: ["Meta", "Tab"] } } }),
  profile("work"),
  profile("valorant", valorant, { KeyQ: { mods: [], action: { type: "key_combo", keys: ["KeyC"] } } }),
  profile("portal-2", portal),
];

describe("game profiles", () => {
  it("finds the games and the Default profile they run on", () => {
    expect(games(list).map((p) => p.id)).toEqual(["valorant", "portal-2"]);
    expect(defaultProfile(list)?.id).toBe(DEFAULT_ID);
    expect(defaultProfile([profile("work")])).toBeNull();
  });

  it("says where a game lives and the name it is watched by", () => {
    expect(gamePlace(valorant)).toBe("C:\\Riot Games\\VALORANT\\live\\VALORANT.exe");
    expect(gamePlace(portal)).toBe("G:\\SteamLibrary\\steamapps\\common\\Portal 2");
    expect(gamePlace({ name: "X", exe: "x.exe", autoSwitch: true })).toBe("x.exe");
    expect(gameExe(valorant)).toBe("VALORANT.exe");
    expect(gameExe(portal)).toBe("Portal 2");
  });

  it("counts bindings in words", () => {
    expect(bindingCount(0)).toBe("0 bindings");
    expect(bindingCount(1)).toBe("1 binding");
    expect(bindingCount(14)).toBe("14 bindings");
  });

  it("gives a name the same tile colours every time", () => {
    expect(tileColors("Valorant")).toEqual(tileColors("Valorant"));
    expect(tileColors("Valorant")[0]).toMatch(/^#[0-9a-f]{6}$/i);
    expect(new Set(["Valorant", "Fortnite", "Minecraft", "Elden Ring", "Rocket League"].map((n) => tileColors(n)[0])).size).toBeGreaterThan(1);
  });

  it("switches automatically only while both switches are on", () => {
    const on = { gameAutoSwitch: true } as Settings;
    expect(switchesAuto(valorant, on)).toBe(true);
    expect(switchesAuto({ ...valorant, autoSwitch: false }, on)).toBe(false);
    expect(switchesAuto(valorant, { gameAutoSwitch: false } as Settings)).toBe(false);
  });

  it("copies the Default's keys in without touching the game's own", () => {
    const game = profile("valorant", valorant, { F3: { mods: [], action: { type: "block" } } });
    const base = profile(DEFAULT_ID, undefined, {
      F3: { mods: [], action: { type: "key_combo", keys: ["Meta", "Tab"] } },
      F4: { mods: [], action: { type: "key_combo", keys: ["Meta"] } },
    });
    expect(copyFromDefault(game, base)).toEqual({
      F3: { mods: [], action: { type: "block" } },
      F4: { mods: [], action: { type: "key_combo", keys: ["Meta"] } },
    });
  });

  it("notes what a remapped key does on its cap", () => {
    expect(capNote({ mods: [], action: { type: "block" } }, [], [])).toBe("Off");
    expect(capNote({ mods: [], action: { type: "key_combo", keys: ["ControlLeft"] } }, [], [])).toBe("Ctrl");
  });

  it("names the type of a binding in one word", () => {
    expect(chipOf({ type: "key_combo", keys: ["KeyC"] })).toBe("Key");
    expect(chipOf({ type: "launch_app", app: { kind: "aumid", id: "x" } })).toBe("App");
    expect(chipOf({ type: "steam_game", appid: 620, name: "Portal 2" })).toBe("Game");
    expect(chipOf({ type: "block" })).toBe("Block");
  });

  it("names the program a picker row adds", () => {
    const e = (kind: "path" | "aumid" | "bundle" | "steam", launch: string) => ({ id: "x", name: "X", kind, launch, icon: null, source: "windows" as const });
    expect(entryExe(e("path", "C:\\Games\\Cyberpunk2077.exe"))).toBe("Cyberpunk2077.exe");
    expect(entryExe(e("aumid", "shell:AppsFolder\\{7C5A40EF-A0FB-4BFC-874A-C0F2E0B9FA8E}\\Tool\\tool.exe"))).toBe("tool.exe");
    expect(entryExe(e("aumid", "shell:AppsFolder\\Microsoft.MinecraftUWP_8wekyb3d8bbwe!App"))).toBe("Store app");
    expect(entryExe(e("steam", "steam://rungameid/730"))).toBe("Steam");
    expect(entryExe(e("bundle", "com.valvesoftware.portal2"))).toBe("com.valvesoftware.portal2");
  });

  it("opens the editor only when a game profile switches in, never on the first load", () => {
    expect(switchedIn(undefined, "valorant", list)).toBeNull();
    expect(switchedIn("valorant", "valorant", list)).toBeNull();
    expect(switchedIn("valorant", "work", list)).toBeNull();
    expect(switchedIn("work", "valorant", list)?.id).toBe("valorant");
  });

  it("tells a detected switch from one made by hand", () => {
    const now = 100_000;
    const ev = { id: "valorant", name: "Valorant", from: "work", game: valorant, at: now - 2000 };
    expect(switchedAuto(ev, "valorant", now)).toBe(true);
    expect(switchedAuto(ev, "portal-2", now)).toBe(false);
    expect(switchedAuto({ ...ev, at: now - 60_000 }, "valorant", now)).toBe(false);
    expect(switchedAuto(null, "valorant", now)).toBe(false);
  });
});
