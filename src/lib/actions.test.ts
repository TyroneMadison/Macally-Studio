import { describe, expect, it } from "vitest";
import {
  describeAction,
  entryGroup,
  entryToAction,
  formatChord,
  isComplete,
  joinArgs,
  KEY_GROUPS,
  keyLabel,
  launchOf,
  modAlias,
  parseChord,
  splitArgs,
} from "./actions";
import { KEY_CODES } from "./keycodes";
import type { AppEntry } from "./types";

const entry = (id: string, kind: AppEntry["kind"], launch: string, name = "App"): AppEntry => ({
  id,
  name,
  kind,
  launch,
  icon: null,
  source: "windows",
});

describe("chords", () => {
  it("maps modifier codes and aliases", () => {
    expect(modAlias("ControlRight")).toBe("Ctrl");
    expect(modAlias("MetaLeft")).toBe("Meta");
    expect(modAlias("cmd")).toBe("Meta");
    expect(modAlias("Option")).toBe("Alt");
    expect(modAlias("KeyA")).toBeNull();
    expect(modAlias("esc")).toBeNull();
  });

  it("parses typed chords into key ids", () => {
    expect(parseChord("Win + Tab")).toEqual(["Meta", "Tab"]);
    expect(parseChord("ctrl+shift+s")).toEqual(["Ctrl", "Shift", "KeyS"]);
    expect(parseChord("f13")).toEqual(["F13"]);
    expect(parseChord("Alt + 4")).toEqual(["Alt", "Digit4"]);
    expect(parseChord("esc")).toEqual(["Escape"]);
    expect(parseChord("pageup")).toEqual(["PageUp"]);
    expect(parseChord(" + ")).toEqual([]);
  });

  it("formats chords per OS and round-trips", () => {
    expect(formatChord(["ControlLeft", "KeyZ"], false)).toBe("Ctrl + Z");
    expect(formatChord(["Meta", "Tab"], false)).toBe("Win + Tab");
    expect(formatChord(["Meta", "Alt", "Escape"], true)).toBe("Cmd + Option + Esc");
    expect(parseChord(formatChord(["Shift", "Digit1", "F5"], false))).toEqual(["Shift", "Digit1", "F5"]);
    expect(parseChord(formatChord(["Ctrl", "Alt", "Delete"], true))).toEqual(["Ctrl", "Alt", "Delete"]);
  });
});

describe("key list", () => {
  it("lists every key Windows can send exactly once", () => {
    const unsendable = ["Fn", "Eject", "BrightnessDown", "BrightnessUp"]; // tests run as Windows (isMac false)
    const listed = KEY_GROUPS.flatMap((g) => g.codes);
    expect(listed).toHaveLength(KEY_CODES.length - unsendable.length);
    expect(new Set(listed)).toEqual(new Set(KEY_CODES.filter((c) => !unsendable.includes(c))));
  });

  it("groups and labels keys the way people look for them", () => {
    const group = (code: string) => KEY_GROUPS.find((g) => g.codes.includes(code))?.label;
    expect(KEY_GROUPS.map((g) => g.label)).toEqual([
      "Letters", "Numbers", "Function keys", "Modifiers", "Navigation & editing", "Numpad", "Media & volume", "Symbols",
    ]);
    expect(KEY_GROUPS[0].codes.slice(0, 3)).toEqual(["KeyA", "KeyB", "KeyC"]);
    expect(KEY_GROUPS[2].codes).toHaveLength(15);
    expect(["ShiftRight", "AltRight", "Escape", "ArrowUp", "NumLock", "AudioVolumeUp", "Semicolon"].map(group)).toEqual([
      "Modifiers", "Modifiers", "Navigation & editing", "Navigation & editing", "Numpad", "Media & volume", "Symbols",
    ]);
    expect(keyLabel("ControlRight", false)).toBe("Right Ctrl");
    expect(keyLabel("MetaLeft", true)).toBe("Left Cmd");
    expect(keyLabel("Semicolon", false)).toBe("Semicolon ;");
    expect(keyLabel("MediaPlayPause", false)).toBe("Play/Pause");
    expect(keyLabel("KeyQ", false)).toBe("Q");
  });
});

describe("picker helpers", () => {
  it("treats unfilled actions as incomplete", () => {
    expect(isComplete({ type: "key_combo", keys: [] })).toBe(false);
    expect(isComplete({ type: "key_combo", keys: ["Ctrl", "Sfit"] })).toBe(false);
    expect(isComplete({ type: "key_combo", keys: ["Win", "Shift", "KeyS"] })).toBe(true);
    expect(isComplete({ type: "key_combo", keys: ["AudioVolumeUp"] })).toBe(true);
    expect(isComplete({ type: "launch_app", app: { kind: "path", path: "", args: "" } })).toBe(false);
    expect(isComplete({ type: "steam_game", appid: 0, name: "" })).toBe(false);
    expect(isComplete({ type: "switch_profile", profile: "" })).toBe(false);
    expect(isComplete({ type: "open", target: "https://github.com" })).toBe(true);
    expect(isComplete({ type: "pass_through" })).toBe(true);
  });

  it("splits and joins quoted args", () => {
    const args = splitArgs('-File "C:\\My Scripts\\x.ps1" -NoLogo');
    expect(args).toEqual(["-File", "C:\\My Scripts\\x.ps1", "-NoLogo"]);
    expect(splitArgs(joinArgs(args))).toEqual(args);
    expect(splitArgs("   ")).toEqual([]);
  });
});

describe("catalog entries", () => {
  const calc = entry(
    "builtin:shell:AppsFolder\\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App",
    "aumid",
    "shell:AppsFolder\\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App",
    "Calculator",
  );
  const portal = entry("steam:620", "steam", "steam://rungameid/620", "Portal 2");
  const notes = entry("builtin:com.apple.Notes", "bundle", "com.apple.Notes");
  const exe = entry("app:C:\\Tools\\x.exe", "path", "C:\\Tools\\x.exe");

  it("turns tiles into actions", () => {
    expect(entryToAction(calc)).toEqual({
      type: "launch_app",
      app: { kind: "aumid", id: "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App" },
    });
    expect(entryToAction(portal)).toEqual({ type: "steam_game", appid: 620, name: "Portal 2" });
    expect(entryToAction(notes)).toEqual({ type: "launch_app", app: { kind: "bundle", id: "com.apple.Notes" } });
    expect(entryToAction(exe)).toEqual({ type: "launch_app", app: { kind: "path", path: "C:\\Tools\\x.exe", args: "" } });
  });

  it("launchOf inverts entryToAction", () => {
    for (const e of [calc, portal, notes, exe]) expect(launchOf(entryToAction(e))).toBe(e.launch);
    expect(launchOf({ type: "pass_through" })).toBeNull();
    expect(launchOf(undefined)).toBeNull();
  });

  it("reads the group from the id prefix", () => {
    expect([calc, portal, notes, exe].map(entryGroup)).toEqual(["builtin", "steam", "builtin", "installed"]);
  });

  it("describes bindings with catalog and profile names", () => {
    expect(describeAction(entryToAction(calc), [calc], []).title).toBe("Calculator");
    expect(describeAction({ type: "key_combo", keys: ["Meta", "Tab"] }, [], [], false).title).toBe("Win + Tab");
    expect(describeAction({ type: "switch_profile", profile: "gaming" }, [], []).title).toBe("gaming");
    expect(describeAction(undefined, [], []).title).toBe("Not remapped");
    expect(describeAction({ type: "block" }, [], [])).toEqual({ type: "Block", title: "Disabled", detail: "The key does nothing" });
  });
});

describe("friendly key names", () => {
  it("shows media keys by name and reads them back", () => {
    expect(formatChord(["AudioVolumeMute"], false)).toBe("Mute");
    expect(formatChord(["Meta", "Shift", "KeyS"], false)).toBe("Win + Shift + S");
    expect(parseChord(formatChord(["MediaTrackPrevious"], false))).toEqual(["MediaTrackPrevious"]);
    expect(parseChord("volume up")).toEqual(["AudioVolumeUp"]);
  });
});
