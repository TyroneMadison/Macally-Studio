import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import ActionPicker from "./ActionPicker";
import { entryToAction } from "../lib/actions";
import type { Action, AppEntry, Profile } from "../lib/types";

const calc: AppEntry = {
  id: "builtin:shell:AppsFolder\\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App",
  name: "Calculator",
  kind: "aumid",
  launch: "shell:AppsFolder\\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App",
  icon: null,
  source: "windows",
};
const zip: AppEntry = {
  id: "app:shell:AppsFolder\\{6D809377-6AF0-444B-8957-A3773F02200E}\\7-Zip\\7zFM.exe",
  name: "7-Zip File Manager",
  kind: "aumid",
  launch: "shell:AppsFolder\\{6D809377-6AF0-444B-8957-A3773F02200E}\\7-Zip\\7zFM.exe",
  icon: null,
  source: "windows",
};
const portal: AppEntry = {
  id: "steam:620",
  name: "Portal 2",
  kind: "steam",
  launch: "steam://rungameid/620",
  icon: null,
  source: "steam",
};
const gaming: Profile = {
  schema: 1,
  id: "gaming",
  name: "Gaming",
  builtin: false,
  hotkey: null,
  autoApps: [],
  bindings: {},
};

const render = (initial?: Action, open = true) =>
  renderToStaticMarkup(
    <ActionPicker
      open={open}
      keyCode="F14"
      initial={initial}
      catalog={[calc, zip, portal]}
      profiles={[gaming]}
      onPick={() => {}}
      onClose={() => {}}
    />,
  );

describe("ActionPicker", () => {
  it("renders nothing while closed", () => {
    expect(render(undefined, false)).toBe("");
  });

  it("opens on the App tab with the bound app selected and all three sections", () => {
    const html = render(entryToAction(calc));
    expect(html).toContain("Choose what F14 does");
    expect(html).toContain('id="ap-tab-app" aria-selected="true"');
    expect(html).toContain("Windows built-in");
    expect(html).toContain("Installed apps");
    expect(html).toContain("Steam library");
    expect(html).toContain('class="ap-tile on" aria-pressed="true" title="Calculator"');
    expect(html).toContain("Microsoft.WindowsCalculator_8wekyb3d8bbwe!App</code>");
    expect(html).toContain('<button type="button" class="btn primary">Bind to F14</button>');
  });

  it("shows only the Steam section on the Steam tab and waits for a pick", () => {
    const html = render({ type: "steam_game", appid: 0, name: "" });
    expect(html).toContain('id="ap-tab-steam" aria-selected="true"');
    expect(html).toContain("Portal 2");
    expect(html).not.toContain("Calculator");
    expect(html).toContain("Nothing selected yet");
    expect(html).toContain('class="btn primary" disabled="">Bind to F14');
  });

  it("prefills the chord, the command and the profile list", () => {
    expect(render({ type: "key_combo", keys: ["Meta", "Tab"] })).toContain('value="Win + Tab"');
    const cmd = render({ type: "command", program: "powershell", args: ["-File", "C:\\My Scripts\\x.ps1"], hidden: true });
    expect(cmd).toContain('value="-File &quot;C:\\My Scripts\\x.ps1&quot;"');
    expect(render({ type: "switch_profile", profile: "gaming" })).toContain('aria-checked="true" class="ap-item on">Gaming');
  });
});
