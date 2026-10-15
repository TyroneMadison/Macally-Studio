import { describe, expect, it } from "vitest";
import { actionLabel, initials, newProfile, ruleFor, uniqueId, withBinding } from "./profile";
import type { AppEntry, Binding } from "./types";

const app = (kind: AppEntry["kind"], launch: string, name = "App"): AppEntry => ({
  id: launch,
  name,
  kind,
  launch,
  icon: null,
  source: "windows",
});

describe("profile helpers", () => {
  it("uniqueId counts up from 2", () => {
    expect(uniqueId("a", [])).toBe("a");
    expect(uniqueId("a", ["a", "a-2"])).toBe("a-3");
  });

  it("newProfile is empty, editable and unique", () => {
    const first = newProfile([]);
    expect(first).toMatchObject({ id: "new-profile", builtin: false, hotkey: null, autoApps: [], bindings: {} });
    expect(newProfile([first]).id).toBe("new-profile-2");
  });

  it("withBinding rebinds one key from its current binding and unbinds on null", () => {
    const q: Binding = { mods: ["Shift"], action: { type: "key_combo", keys: ["KeyQ"] } };
    const p = { ...newProfile([]), bindings: { F13: q, F14: q } };
    const tab = withBinding(p, "F13", (old) => ({ mods: old?.mods ?? [], action: { type: "key_combo", keys: ["Tab"] } }));
    expect(tab.bindings).toEqual({ F13: { mods: ["Shift"], action: { type: "key_combo", keys: ["Tab"] } }, F14: q });
    expect(withBinding(p, "F15", (old) => old ?? null).bindings).toEqual(p.bindings);
    expect(withBinding(p, "F14", () => null).bindings).toEqual({ F13: q });
    expect(p.bindings).toEqual({ F13: q, F14: q }); // the input is left alone
  });

  it("initials", () => {
    expect(initials("Photo editing")).toBe("PE");
    expect(initials("gaming")).toBe("GA");
    expect(initials("  ")).toBe("?");
  });

  it("ruleFor uses the exe name or bundle id", () => {
    expect(ruleFor(app("aumid", "{6D809377-6AF0-444B-8957-A3773F02200E}\\Adobe\\Photoshop.exe"))).toBe("Photoshop.exe");
    expect(ruleFor(app("path", "C:\\Games\\game.EXE"))).toBe("game.EXE");
    expect(ruleFor(app("bundle", "com.adobe.Lightroom"))).toBe("com.adobe.Lightroom");
    expect(ruleFor(app("aumid", "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"))).toBeNull();
    expect(ruleFor(app("steam", "620"))).toBeNull();
  });

  it("actionLabel", () => {
    const calc = app("aumid", "shell:AppsFolder\\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App", "Calculator");
    expect(actionLabel({ type: "key_combo", keys: ["Win", "Tab"] }, [], [])).toBe("Win + Tab");
    expect(actionLabel({ type: "block" }, [], [])).toBe("Disabled");
    expect(
      actionLabel({ type: "launch_app", app: { kind: "aumid", id: "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App" } }, [calc], []),
    ).toBe("Calculator");
    expect(actionLabel({ type: "launch_app", app: { kind: "path", path: "C:\\x\\tool.exe", args: "" } }, [], [])).toBe("tool.exe");
    expect(actionLabel({ type: "switch_profile", profile: "new-profile" }, [], [newProfile([])])).toBe("Switch to New profile");
    expect(actionLabel({ type: "switch_profile", profile: "gone" }, [], [])).toBe("Switch to gone");
    expect(actionLabel({ type: "pass_through" }, [], [])).toBe("Pass-through");
  });
});
