import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import Inspector from "./Inspector";
import type { AppEntry, Layout, Profile } from "../lib/types";

const calc: AppEntry = {
  id: "builtin:shell:AppsFolder\\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App",
  name: "Calculator",
  kind: "aumid",
  launch: "shell:AppsFolder\\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App",
  icon: null,
  source: "windows",
};
const layout: Layout = {
  unit: 19.05,
  keys: [
    { code: "F14", row: 0, x: 16.5, w: 1, h: 0.6, mac: "F14", win: "F14" },
    { code: "F3", row: 0, x: 3.21, w: 1.07, h: 0.6, mac: "F3", win: "F3" },
  ],
};
const mine: Profile = {
  schema: 1,
  id: "mine",
  name: "Mine",
  builtin: false,
  hotkey: null,
  autoApps: [],
  bindings: {
    F14: {
      mods: ["Shift"],
      action: { type: "launch_app", app: { kind: "aumid", id: "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App" } },
    },
    F3: { mods: [], action: { type: "key_combo", keys: ["Meta", "Tab"] } },
  },
};

const render = (keyCode: string | null, profile: Profile = mine) =>
  renderToStaticMarkup(
    <Inspector
      keyCode={keyCode}
      layout={layout}
      profile={profile}
      onChange={() => {}}
      catalog={[calc]}
      profiles={[profile]}
      lastKey={null}
    />,
  );

describe("Inspector", () => {
  it("asks for a key when nothing is selected and still shows the monitor", () => {
    const html = render(null);
    expect(html).toContain("Click a key to inspect it.");
    expect(html).toContain("Press any key to see what the OS reports");
  });

  it("summarises the saved binding and its held-with modifier", () => {
    const html = render("F14");
    expect(html).toContain("Currently bound to");
    expect(html).toContain('<span class="insp-title">Calculator</span>');
    expect(html).toContain('<option value="Shift" selected="">Shift</option>');
    expect(html).toContain('aria-pressed="true">App</button>');
    expect(html).toContain('class="btn primary" disabled="">Save binding');
  });

  it("edits a key combo inline", () => {
    const html = render("F3");
    expect(html).toContain('<span class="insp-title">Win + Tab</span>');
    expect(html).toContain('id="insp-chord"');
    expect(html).toContain('value="Win + Tab"');
  });

  it("locks editing on built-in profiles", () => {
    const html = render("F14", { ...mine, builtin: true });
    expect(html).toContain("Built-in profiles are read-only");
    expect(html).toContain('<select id="insp-held" class="input" disabled="">');
  });
});
