import { describe, expect, it } from "vitest";
import { formatHotkey } from "./hotkey";
import type { KeyEvent } from "./types";

const ev = (code: string, mods: string[], down = true): KeyEvent => ({ code, mods, down, raw: "" });

describe("formatHotkey", () => {
  it("orders modifiers and shortens letter and digit codes", () => {
    expect(formatHotkey(ev("KeyX", ["Alt", "Ctrl"]))).toBe("Ctrl+Alt+X");
    expect(formatHotkey(ev("Digit2", ["ControlLeft", "AltRight"]))).toBe("Ctrl+Alt+2");
    expect(formatHotkey(ev("F13", ["MetaLeft", "Shift"]))).toBe("Shift+Super+F13");
    expect(formatHotkey(ev("BracketRight", ["Ctrl", "Alt"]))).toBe("Ctrl+Alt+BracketRight");
  });

  it("waits for a real key: ignores modifier-only presses and key-ups", () => {
    expect(formatHotkey(ev("ControlLeft", ["Ctrl"]))).toBeNull();
    expect(formatHotkey(ev("KeyP", ["Ctrl"], false))).toBeNull();
  });
});
