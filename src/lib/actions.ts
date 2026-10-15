import { KEY_CODES } from "./keycodes";
import type { Action, AppEntry, Profile } from "./types";

export const isMac = typeof navigator !== "undefined" && /Macintosh|Mac OS X/.test(navigator.userAgent);

export type Mod = "Ctrl" | "Alt" | "Shift" | "Meta";
/** Label order: Control, Option, Shift, Command on a Mac; Win first on Windows ("Win + Shift + S"). */
const MOD_ORDER: Mod[] = isMac ? ["Ctrl", "Alt", "Shift", "Meta"] : ["Meta", "Ctrl", "Alt", "Shift"];

/** Readable names for keys whose code is not what people call them. formatChord shows these; parseChord reads them back. */
const FRIENDLY: Record<string, string> = {
  MediaTrackPrevious: "Previous track", MediaPlayPause: "Play/Pause", MediaTrackNext: "Next track",
  AudioVolumeMute: "Mute", AudioVolumeDown: "Volume down", AudioVolumeUp: "Volume up",
  BrightnessDown: "Brightness down", BrightnessUp: "Brightness up", ContextMenu: "Menu", Escape: "Esc",
  ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left", ArrowRight: "Right", PageUp: "Page up", PageDown: "Page down",
  PrintScreen: "Print screen", NumLock: "Num lock", CapsLock: "Caps lock",
};

/** Typed names -> key ids the engine accepts (modifier aliases or W3C codes). */
const NAMES: Record<string, string> = {
  ...Object.fromEntries(Object.entries(FRIENDLY).map(([code, name]) => [name.toLowerCase(), code])),
  ctrl: "Ctrl", control: "Ctrl", alt: "Alt", opt: "Alt", option: "Alt", shift: "Shift",
  meta: "Meta", win: "Meta", cmd: "Meta", command: "Meta",
  esc: "Escape", return: "Enter", del: "Delete", menu: "ContextMenu", pgup: "PageUp", pgdn: "PageDown",
  up: "ArrowUp", down: "ArrowDown", left: "ArrowLeft", right: "ArrowRight",
};

/** "ControlRight" -> "Ctrl", "cmd" -> "Meta"; null when the name is not a modifier. */
export function modAlias(name: string): Mod | null {
  const m = /^(Control|Alt|Shift|Meta)(Left|Right)$/.exec(name);
  if (m) return m[1] === "Control" ? "Ctrl" : (m[1] as Mod);
  const alias = NAMES[name.toLowerCase()];
  return MOD_ORDER.includes(alias as Mod) ? (alias as Mod) : null;
}

/** "ctrl + shift + s" -> ["Ctrl", "Shift", "KeyS"]. Letters/digits become codes, code names match case-insensitively. */
export function parseChord(text: string): string[] {
  return text
    .split("+")
    .map((t) => t.trim())
    .filter(Boolean)
    .map((t) => {
      const lower = t.toLowerCase();
      if (NAMES[lower]) return NAMES[lower];
      if (/^[a-z]$/.test(lower)) return `Key${t.toUpperCase()}`;
      if (/^[0-9]$/.test(t)) return `Digit${t}`;
      return KEY_CODES.find((c) => c.toLowerCase() === lower) ?? t;
    });
}

/** ["ControlLeft", "KeyZ"] -> "Ctrl + Z" ("Control + Z" on a Mac). The output parses back with parseChord. */
export function formatChord(keys: string[], mac = isMac): string {
  const label: Record<Mod, string> = mac
    ? { Ctrl: "Control", Alt: "Option", Shift: "Shift", Meta: "Cmd" }
    : { Ctrl: "Ctrl", Alt: "Alt", Shift: "Shift", Meta: "Win" };
  return keys
    .map((k) => {
      const m = modAlias(k);
      return m ? label[m] : FRIENDLY[k] ?? k.replace(/^Key([A-Z])$/, "$1").replace(/^Digit(\d)$/, "$1");
    })
    .join(" + ");
}

const SYMBOLS: Record<string, string> = {
  Backquote: "`", Minus: "-", Equal: "=", BracketLeft: "[", BracketRight: "]", Backslash: "\\",
  Semicolon: ";", Quote: "'", Comma: ",", Period: ".", Slash: "/",
};

function groupOf(code: string): string {
  if (/^Key[A-Z]$/.test(code)) return "Letters";
  if (/^Digit\d$/.test(code)) return "Numbers";
  if (/^F\d+$/.test(code)) return "Function keys";
  if (code === "Fn" || modAlias(code)) return "Modifiers";
  if (code.startsWith("Num")) return "Numpad";
  if (/^(Media|Audio|Brightness|Launch|Eject)/.test(code)) return "Media & volume";
  return SYMBOLS[code] ? "Symbols" : "Navigation & editing";
}

/** Keys this OS cannot send (engine can_send is false); keymap.rs checks this line. */
const UNSENDABLE = isMac ? ["LaunchApp1", "PrintScreen", "ScrollLock", "Pause"] : ["Fn", "Eject", "BrightnessDown", "BrightnessUp"];

/** Every key the engine can send on this OS, grouped for the Inspector's "Remap to" list. */
export const KEY_GROUPS = [
  "Letters", "Numbers", "Function keys", "Modifiers", "Navigation & editing", "Numpad", "Media & volume", "Symbols",
].map((label) => {
  const codes = KEY_CODES.filter((c) => groupOf(c) === label && !UNSENDABLE.includes(c));
  return { label, codes: label === "Letters" ? codes.sort() : codes }; // A-Z, not QWERTY order
});

/** One key's list label: "Left Ctrl", "Semicolon ;", "Play/Pause". */
export function keyLabel(code: string, mac = isMac): string {
  const side = /^(?:Control|Alt|Shift|Meta)(Left|Right)$/.exec(code)?.[1];
  const name = formatChord([code], mac);
  return side ? `${side} ${name}` : SYMBOLS[code] ? `${name} ${SYMBOLS[code]}` : name;
}

/** `-File "C:\My Scripts\x.ps1"` -> ["-File", "C:\My Scripts\x.ps1"]. */
export function splitArgs(text: string): string[] {
  return (text.match(/"[^"]*"|\S+/g) ?? []).map((s) => s.replace(/^"(.*)"$/, "$1"));
}

/** Inverse of splitArgs, for showing saved args in a text field. */
export function joinArgs(args: string[]): string {
  return args.map((a) => (/\s/.test(a) ? `"${a}"` : a)).join(" ");
}

/** False for an action the picker has not filled in yet (no keys, no app, empty target...). */
export function isComplete(a: Action): boolean {
  switch (a.type) {
    case "key_combo":
      // a typo ("Sfit") would save, light the key, then be dropped by the engine
      return a.keys.length > 0 && a.keys.every((k) => modAlias(k) !== null || KEY_CODES.includes(k));
    case "launch_app":
      return (a.app.kind === "path" ? a.app.path : a.app.id).trim() !== "";
    case "steam_game":
      return a.appid > 0;
    case "open":
      return a.target.trim() !== "";
    case "command":
      return a.program.trim() !== "";
    case "switch_profile":
      return a.profile !== "";
    default:
      return true;
  }
}

const APPS_FOLDER = "shell:AppsFolder\\";

/** Catalog tile -> the action that launches it. */
export function entryToAction(e: AppEntry): Action {
  switch (e.kind) {
    case "aumid":
      return {
        type: "launch_app",
        app: { kind: "aumid", id: e.launch.startsWith(APPS_FOLDER) ? e.launch.slice(APPS_FOLDER.length) : e.launch },
      };
    case "bundle":
      return { type: "launch_app", app: { kind: "bundle", id: e.launch } };
    case "steam":
      return { type: "steam_game", appid: Number(e.launch.split("/").pop()), name: e.name };
    default:
      return { type: "launch_app", app: { kind: "path", path: e.launch, args: "" } };
  }
}

/** Inverse of entryToAction: the catalog launch string an action points at, or null. */
export function launchOf(a: Action | null | undefined): string | null {
  if (a?.type === "steam_game") return `steam://rungameid/${a.appid}`;
  if (a?.type !== "launch_app") return null;
  if (a.app.kind === "aumid") return APPS_FOLDER + a.app.id;
  return a.app.kind === "bundle" ? a.app.id : a.app.path;
}

export type Group = "builtin" | "installed" | "steam";

/** Catalog ids are "<group>:<launch>" (catalog/mod.rs): builtin: / app: / steam:. */
export function entryGroup(e: AppEntry): Group {
  const prefix = e.id.slice(0, e.id.indexOf(":"));
  return prefix === "builtin" || prefix === "steam" ? prefix : "installed";
}

export type ActionSummary = { type: string; title: string; detail: string };

/** Human summary of an action for the Inspector card and the picker footer. */
export function describeAction(
  a: Action | undefined,
  catalog: AppEntry[],
  profiles: Profile[],
  mac = isMac,
): ActionSummary {
  if (!a) return { type: "Pass-through", title: "Not remapped", detail: "Sends its own key" };
  switch (a.type) {
    case "key_combo":
      return { type: "Key / Combo", title: formatChord(a.keys, mac), detail: a.keys.join(" + ") };
    case "launch_app":
    case "steam_game": {
      const launch = launchOf(a) ?? "";
      const known = catalog.find((e) => e.launch === launch)?.name;
      const fallback = a.type === "steam_game" ? a.name : (launch.split(/[\\/]/).pop() ?? launch);
      return { type: a.type === "steam_game" ? "Steam game" : "App", title: known ?? fallback, detail: launch };
    }
    case "open":
      return { type: "URL / File", title: a.target, detail: "Opens with the default app" };
    case "command":
      return { type: "Command", title: a.program, detail: joinArgs(a.args) };
    case "switch_profile":
      return {
        type: "Profile",
        title: profiles.find((p) => p.id === a.profile)?.name ?? a.profile,
        detail: "Switches the active profile",
      };
    case "pause_toggle":
      return { type: "Pause", title: "Pause / resume remapping", detail: "" };
    case "pass_through":
      return { type: "Pass-through", title: "Sends its own key", detail: "" };
  }
}
