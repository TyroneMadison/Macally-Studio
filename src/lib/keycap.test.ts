import { describe, expect, it } from "vitest";
import { KEY_CODES } from "./keycodes";
import { buildChord, capFace, keyMatches, WINDOWS_SHORTCUTS } from "./keycap";
import { isComplete } from "./actions";

describe("capFace", () => {
  it("draws every key with a legend or an icon", () => {
    for (const code of KEY_CODES) {
      for (const mac of [false, true]) {
        const f = capFace(code, mac);
        expect(f.legend !== "" || f.icon !== undefined, `${code} mac=${mac}`).toBe(true);
      }
    }
  });

  it("puts the Windows logo on Win and the command loop on a Mac", () => {
    for (const code of ["MetaLeft", "MetaRight", "Meta", "Win", "Cmd"]) {
      expect(capFace(code, false)).toEqual({ legend: "", icon: "windows" });
      expect(capFace(code, true)).toEqual({ legend: "cmd", icon: "command" });
    }
    expect(capFace("AltLeft", true).icon).toBe("option");
    expect(capFace("AltLeft", false)).toEqual({ legend: "alt" });
  });

  it("names keys the way their caps do", () => {
    expect(capFace("KeyV", false).legend).toBe("V");
    expect(capFace("Digit4", false).legend).toBe("4");
    expect(capFace("Numpad7", false).legend).toBe("7");
    expect(capFace("F13", false).legend).toBe("F13");
    expect(capFace("Enter", true).legend).toBe("return");
    expect(capFace("Backspace", false)).toEqual({ legend: "backspace", wide: true });
    expect(capFace("Space", false).wide).toBe(true);
    expect(capFace("AudioVolumeUp", false).icon).toBe("vol-up");
    expect(capFace("MediaPlayPause", false).icon).toBe("play-pause");
    expect(capFace("ArrowLeft", false).legend).toBe("←");
  });
});

describe("buildChord", () => {
  it("puts the send-with modifiers first, Win leading on Windows", () => {
    expect(buildChord(["Shift", "Meta"], "KeyS", false)).toEqual(["Meta", "Shift", "KeyS"]);
    expect(buildChord(["Meta"], "KeyV", false)).toEqual(["Meta", "KeyV"]);
    expect(buildChord([], "MetaLeft", false)).toEqual(["MetaLeft"]);
  });

  it("does not repeat a modifier that is also the picked key", () => {
    expect(buildChord(["Meta", "Ctrl"], "MetaRight", false)).toEqual(["Ctrl", "MetaRight"]);
  });
});

describe("keyMatches", () => {
  it("finds the Win key by its many names", () => {
    for (const q of ["win", "Windows", "cmd", "super"]) expect(keyMatches("MetaLeft", "Left Win", q)).toBe(true);
    expect(keyMatches("KeyV", "V", "win")).toBe(false);
    expect(keyMatches("KeyV", "V", "")).toBe(true);
  });
});

describe("WINDOWS_SHORTCUTS", () => {
  it("are all valid chords led by the Win key", () => {
    for (const s of WINDOWS_SHORTCUTS) {
      expect(isComplete({ type: "key_combo", keys: s.keys }), s.label).toBe(true);
      expect(s.keys[0]).toBe("Meta");
    }
    expect(WINDOWS_SHORTCUTS.find((s) => s.label === "Clipboard history")?.keys).toEqual(["Meta", "KeyV"]);
  });
});
