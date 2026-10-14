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
      saveBinding={async () => {}}
      catalog={[calc]}
      profiles={[profile]}
      lastKey={null}
    />,
  );

describe("Inspector", () => {
  it("explains both ways to pick a key and still shows the monitor", () => {
    const html = render(null);
    expect(html).toContain("Click a key on the keyboard, or press &#x27;Pick key by pressing&#x27; and press it.");
    expect(html).toContain("Press any key to see what the OS reports");
  });

  it("summarises the saved binding and its held-with modifier", () => {
    const html = render("F14");
    expect(html).toContain("Currently bound to");
    expect(html).toContain('<span class="insp-title">Calculator</span>');
    expect(html).toContain('<option value="Shift" selected="">Shift</option>');
    expect(html).toContain(">More actions…</button>");
    expect(html).toContain('<button type="button" class="btn">Reset key</button>');
  });

  it("offers Press a key and the keycap list, showing what the key sends", () => {
    const html = render("F3", { ...mine, bindings: { F3: { mods: [], action: { type: "key_combo", keys: ["KeyQ"] } } } });
    expect(html).toContain('<button type="button" class="btn primary" aria-pressed="false">Press a key</button>');
    expect(html).toContain('aria-label="Sends Q. Choose a key"');
    expect(html).toContain('data-code="KeyQ"');
  });

  it("shows a Win chord as keycaps with the Windows logo", () => {
    const html = render("F3");
    expect(html).toContain('<span class="insp-title">Win + Tab</span>');
    expect(html).toContain('aria-label="Sends Win + Tab. Choose a key"');
    expect(html).toContain('data-icon="windows"');
  });

  it("offers keys and programs on its prompt for an unbound key", () => {
    const html = render("F3", { ...mine, bindings: {} });
    expect(html).toContain('aria-label="Choose a key"');
    expect(html).toContain("Choose a key or program…");
  });

  it("shows a program binding as the program it opens", () => {
    const html = render("F14");
    expect(html).toContain('aria-label="Launches Calculator. Choose a key or program"');
    expect(html).toContain("Open Calculator");
  });

  it("edits built-in profiles like any other", () => {
    const html = render("F14", { ...mine, builtin: true });
    expect(html).not.toContain("read-only");
    expect(html).not.toMatch(/<(button|select)[^>]* disabled=""/);
  });

  it("has nothing to reset or hold on an unbound key", () => {
    const html = render("F3", { ...mine, bindings: {} });
    expect(html).toContain("Not remapped");
    expect(html).toContain('<select id="insp-held" class="input" disabled="">');
    expect(html).toContain('<button type="button" class="btn" disabled="">Reset key</button>');
  });
});
