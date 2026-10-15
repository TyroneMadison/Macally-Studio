// Game profiles in the UI: a profile with a `game` is that game's own; it switches in while the game
// is in front (foreground.rs) and runs on top of the Default profile (AppState::engine_profile).
import { isMac } from "./actions";
import { actionLabel } from "./profile";
import type { Action, AppEntry, Binding, Game, Profile, Settings, Switched } from "./types";

/** The built-in profile for this OS: what runs when no game does, and under every game's own keys. */
export const DEFAULT_ID = isMac ? "macos-default" : "windows-default";

export const games = (profiles: Profile[]) => profiles.filter((p) => p.game);

export const defaultProfile = (profiles: Profile[]) => profiles.find((p) => p.id === DEFAULT_ID) ?? null;

/** Where a game lives, for its header: the exe's path, a Steam game's install folder, else its exe. */
export const gamePlace = (g: Game) => g.path ?? g.folder ?? g.exe;

/** The name a game is watched by: its exe, or a Steam game's install folder name. */
export const gameExe = (g: Game) => g.exe || (g.folder?.split(/[\\/]/).filter(Boolean).pop() ?? "");

export const bindingCount = (n: number) => `${n} binding${n === 1 ? "" : "s"}`;

/** Gradient stops of letter tiles, the mockups' red, violet, green, gold, teal, orange and blue. */
const TILES: [string, string][] = [
  ["#ff5a5f", "#c81e3a"],
  ["#8b7bff", "#4f46e5"],
  ["#4ade80", "#15803d"],
  ["#e0b04a", "#a16207"],
  ["#2dd4bf", "#0e7490"],
  ["#fb923c", "#c2410c"],
  ["#60a5fa", "#1d4ed8"],
];

/** A name's tile colours: always the same for the same name. */
export function tileColors(name: string): [string, string] {
  let h = 0;
  for (const c of name) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return TILES[h % TILES.length];
}

/** The game's profile switches in by itself: its own switch and the one for every game are on. */
export const switchesAuto = (g: Game, s: Settings) => s.gameAutoSwitch && g.autoSwitch;

/** "Copy from Default": the Default profile's keys added under the game's own, which win. */
export const copyFromDefault = (game: Profile, base: Profile): Record<string, Binding> => ({ ...base.bindings, ...game.bindings });

const CHIP: Record<Action["type"], string> = {
  key_combo: "Key",
  launch_app: "App",
  steam_game: "Game",
  open: "Open",
  command: "Command",
  switch_profile: "Profile",
  pass_through: "Own key",
  block: "Block",
};

/** A binding's kind in one word, for the bindings list. */
export const chipOf = (a: Action) => CHIP[a.type];

/** What a picker row adds, by the name it is watched by: the exe, a bundle id, Steam, or a Store app. */
export function entryExe(e: AppEntry): string {
  if (e.kind === "steam") return "Steam";
  if (e.kind === "bundle") return e.launch;
  const file = e.launch.split(/[\\/]/).pop() ?? e.launch;
  return /\.(exe|lnk)$/i.test(file) ? file : "Store app";
}

/** What a remapped key does, short enough for its cap on the editor's keyboard. */
export function capNote(b: Binding, catalog: AppEntry[], profiles: Profile[]): string {
  return b.action.type === "block" ? "Off" : actionLabel(b.action, catalog, profiles);
}

/** The game profile that just became active (never on the first load): its full-screen editor opens. */
export function switchedIn(prev: string | undefined, next: string | undefined, profiles: Profile[]): Profile | null {
  if (prev === undefined || next === undefined || prev === next) return null;
  return profiles.find((p) => p.id === next && p.game) ?? null;
}

/** The last automatic switch went to `id` a moment ago: the game was detected, not picked by hand. */
export const switchedAuto = (s: (Switched & { at: number }) | null, id: string, now: number) =>
  !!s && s.id === id && s.game !== null && now - s.at < 10_000;
