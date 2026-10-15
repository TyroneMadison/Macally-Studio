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
      resetKeyEverywhere={async () => 0}
    />,
  );

describe("Inspector", () => {
  it("asks for a click on a key and still shows the monitor", () => {
    const html = render(null);
    expect(html).toContain("Click a key on the keyboard to change it.");
    expect(html).not.toContain("Pick key by pressing");
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

  it("remaps by clicking through the keycap list, never by pressing keys", () => {
    const html = render("F3", { ...mine, bindings: { F3: { mods: [], action: { type: "key_combo", keys: ["KeyQ"] } } } });
    expect(html).not.toContain("Press a key");
    expect(html).not.toContain("aria-pressed");
    expect(html).toContain('aria-label="Sends Q. Choose a key"');
    expect(html).toContain('data-code="KeyQ"');
    expect(html).toContain("Choose what it sends: a key, a Windows shortcut like Win + V, or a program to launch.");
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

  it("turns a key off, and back on", () => {
    expect(render("F14")).toContain(">Turn key off</button>");
    const off = render("F3", { ...mine, bindings: { F3: { mods: [], action: { type: "block" } } } });
    expect(off).toContain('<span class="insp-title">Disabled</span>');
    expect(off).toContain(">Turn key on</button>");
  });

  it("shows the Fn section for the firmware-only Fn key, not the remap controls", () => {
    const fnLayout = { unit: 19.05, keys: [{ code: "Fn", row: 5, x: 0, w: 1, h: 1, mac: "fn", win: "Fn" }] } as unknown as Layout;
    const html = renderToStaticMarkup(
      <Inspector keyCode="Fn" layout={fnLayout} profile={mine} saveBinding={async () => {}} catalog={[calc]} profiles={[mine]} lastKey={null} resetKeyEverywhere={async () => 0} />,
    );
    expect(html).toContain("Fn can’t be remapped");
    expect(html).toContain("Reset Fn key");
    expect(html).not.toContain("Remap to");
    expect(html).not.toContain("Turn key off");
  });

  it("warns that the nav-cluster key printed fn may be a second firmware fn key", () => {
    const navLayout = { unit: 19.05, keys: [{ code: "Insert", row: 1, x: 15.5, w: 1, h: 1, mac: "fn", win: "Insert", verify: true }] } as unknown as Layout;
    const html = renderToStaticMarkup(
      <Inspector keyCode="Insert" layout={navLayout} profile={mine} saveBinding={async () => {}} catalog={[calc]} profiles={[mine]} lastKey={null} resetKeyEverywhere={async () => 0} />,
    );
    expect(html).toContain("second fn key");
    expect(html).toContain("Key Monitor");
    // still remappable in case it does send Insert
    expect(html).toContain("Remap to");
  });

  it("has nothing to reset or hold on an unbound key", () => {
    const html = render("F3", { ...mine, bindings: {} });
    expect(html).toContain("Not remapped");
    expect(html).toContain('<select id="insp-held" class="input" disabled="">');
    expect(html).toContain('<button type="button" class="btn" disabled="">Reset key</button>');
  });
});
