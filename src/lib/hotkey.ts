import type { KeyEvent } from "./types";

const ORDER = ["Ctrl", "Alt", "Shift", "Super"] as const;
type Mod = (typeof ORDER)[number];

// KeyEvent.mods may carry modifier names or left/right key codes; both map here.
// "Super" is the global-shortcut spelling of the Windows / command key.
const MOD_OF: Record<string, Mod> = {
  Ctrl: "Ctrl", ControlLeft: "Ctrl", ControlRight: "Ctrl",
  Alt: "Alt", AltLeft: "Alt", AltRight: "Alt",
  Shift: "Shift", ShiftLeft: "Shift", ShiftRight: "Shift",
  Meta: "Super", MetaLeft: "Super", MetaRight: "Super",
};

/** A non-modifier key-down -> "Ctrl+Alt+X" (global-shortcut syntax). Anything else -> null. */
export function formatHotkey(ev: KeyEvent): string | null {
  if (!ev.down || ev.code in MOD_OF) return null;
  const held = new Set(ev.mods.map((m) => MOD_OF[m]));
  const key = ev.code.replace(/^Key(?=[A-Z]$)/, "").replace(/^Digit(?=\d$)/, "");
  return [...ORDER.filter((m) => held.has(m)), key].join("+");
}
