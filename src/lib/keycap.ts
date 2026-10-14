import { formatChord, isMac, modAlias, type Mod } from "./actions";

/** Pictures drawn on a keycap instead of (or next to) text. */
export type CapIcon =
  | "windows"
  | "command"
  | "option"
  | "play-pause"
  | "prev"
  | "next"
  | "mute"
  | "vol-down"
  | "vol-up"
  | "brightness-down"
  | "brightness-up"
  | "eject";

/** What one key's thumbnail shows: a legend, an icon, or both; wide caps for the long keys. */
export type CapFace = { legend: string; icon?: CapIcon; wide?: boolean };

const ICONS: Record<string, CapIcon> = {
  MediaPlayPause: "play-pause",
  MediaTrackPrevious: "prev",
  MediaTrackNext: "next",
  AudioVolumeMute: "mute",
  AudioVolumeDown: "vol-down",
  AudioVolumeUp: "vol-up",
  BrightnessDown: "brightness-down",
  BrightnessUp: "brightness-up",
  Eject: "eject",
};

const TEXT: Record<string, string> = {
  Escape: "esc", Tab: "tab", CapsLock: "caps", Space: "space", ContextMenu: "menu",
  Insert: "ins", Delete: "del", Home: "home", End: "end", PageUp: "pg up", PageDown: "pg dn",
  ArrowUp: "↑", ArrowDown: "↓", ArrowLeft: "←", ArrowRight: "→",
  PrintScreen: "prt sc", ScrollLock: "scr lk", Pause: "pause", LaunchApp1: "app", Fn: "fn",
  Backquote: "`", Minus: "-", Equal: "=", BracketLeft: "[", BracketRight: "]", Backslash: "\\",
  Semicolon: ";", Quote: "'", Comma: ",", Period: ".", Slash: "/",
  NumpadDivide: "/", NumpadMultiply: "*", NumpadSubtract: "-", NumpadAdd: "+", NumpadDecimal: ".",
  NumpadEqual: "=",
};

const WIDE = new Set(["Space", "Tab", "CapsLock", "Enter", "NumpadEnter", "Backspace", "ShiftLeft", "ShiftRight"]);

/** The modifier a code or alias stands for, or null ("MetaLeft", "Win", "Cmd" -> "Meta"). */
function modOf(code: string): Mod | null {
  return modAlias(code);
}

/** Face of one key, Windows or Mac flavoured: the Win key carries the Windows logo. */
export function capFace(code: string, mac = isMac): CapFace {
  const wide = WIDE.has(code);
  const mod = modOf(code);
  if (mod === "Meta") return mac ? { legend: "cmd", icon: "command" } : { legend: "", icon: "windows" };
  if (mod === "Alt") return mac ? { legend: "opt", icon: "option" } : { legend: "alt" };
  if (mod === "Ctrl") return { legend: mac ? "control" : "ctrl" };
  if (mod === "Shift") return { legend: "shift", wide };
  if (ICONS[code]) return { legend: "", icon: ICONS[code] };
  if (code === "Enter" || code === "NumpadEnter") return { legend: mac && code === "Enter" ? "return" : "enter", wide };
  if (code === "Backspace") return { legend: mac ? "delete" : "backspace", wide };
  if (code === "NumLock") return { legend: mac ? "clear" : "num" };
  if (TEXT[code] !== undefined) return { legend: TEXT[code], wide };
  const digit = /^(?:Digit|Numpad)(\d)$/.exec(code);
  if (digit) return { legend: digit[1] };
  const letter = /^Key([A-Z])$/.exec(code);
  if (letter) return { legend: letter[1] };
  if (/^F\d+$/.test(code)) return { legend: code };
  return { legend: formatChord([code], mac) };
}

/** A chord built in the picker: the "send with" modifiers in label order, then the key. */
export function buildChord(mods: Mod[], key: string, mac = isMac): string[] {
  const order: Mod[] = mac ? ["Ctrl", "Alt", "Shift", "Meta"] : ["Meta", "Ctrl", "Alt", "Shift"];
  const own = modOf(key);
  return [...order.filter((m) => mods.includes(m) && m !== own), key];
}

/** Ready-made Windows shortcuts for the picker; "Meta" is the Win key. */
export const WINDOWS_SHORTCUTS: { keys: string[]; label: string; words: string }[] = [
  { keys: ["Meta"], label: "Start menu", words: "windows win start" },
  { keys: ["Meta", "KeyV"], label: "Clipboard history", words: "paste clipboard history" },
  { keys: ["Meta", "KeyC"], label: "Copilot", words: "copilot assistant" },
  { keys: ["Meta", "KeyE"], label: "File Explorer", words: "files explorer folder" },
  { keys: ["Meta", "KeyD"], label: "Show desktop", words: "desktop hide windows" },
  { keys: ["Meta", "Tab"], label: "Task view", words: "task view windows switch" },
  { keys: ["Meta", "Shift", "KeyS"], label: "Screenshot", words: "snip screenshot capture" },
  { keys: ["Meta", "KeyI"], label: "Settings", words: "settings preferences" },
  { keys: ["Meta", "KeyR"], label: "Run", words: "run command" },
  { keys: ["Meta", "KeyA"], label: "Quick settings", words: "quick settings wifi volume" },
  { keys: ["Meta", "KeyN"], label: "Notifications", words: "notifications calendar" },
  { keys: ["Meta", "Period"], label: "Emoji", words: "emoji symbols" },
];

const MOD_WORDS: Record<Mod, string> = {
  Meta: "win windows cmd command super meta",
  Ctrl: "ctrl control",
  Alt: "alt option opt",
  Shift: "shift",
};

/** True when a key matches the picker's search text (name, code or a common alias). */
export function keyMatches(code: string, label: string, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  const mod = modOf(code);
  return `${label} ${code} ${mod ? MOD_WORDS[mod] : ""}`.toLowerCase().includes(q);
}
