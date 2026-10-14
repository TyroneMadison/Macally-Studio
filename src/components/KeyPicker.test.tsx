import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import KeyPicker from "./KeyPicker";
import { KEY_GROUPS } from "../lib/actions";
import type { Action, AppEntry } from "../lib/types";

const render = (value: string[], mac = false, open = true) =>
  renderToStaticMarkup(<KeyPicker value={value} onPick={() => {}} mac={mac} defaultOpen={open} />);

describe("KeyPicker", () => {
  it("shows the current chord as keycaps on its button", () => {
    const html = render(["Meta", "KeyV"], false, false);
    expect(html).toContain('aria-label="Sends Win + V. Choose a key"');
    expect(html).toContain('data-icon="windows"');
    expect(html).not.toContain("kp-pop");
  });

  it("lists every sendable key exactly once, each as a keycap thumbnail", () => {
    const html = render([]);
    const listed = [...html.matchAll(/<button[^>]*class="kp-key[^"]*"[^>]*data-code="([^"]+)"/g)].map((m) => m[1]);
    expect(listed.sort()).toEqual(KEY_GROUPS.flatMap((g) => g.codes).sort());
    // a thumbnail per key: the button wraps a cap for the same code
    expect(html).toMatch(/data-code="MetaLeft"[^>]*>.*?<span class="cap cap-md" data-code="MetaLeft"/);
  });

  it("offers Win as a send-with modifier and the Windows shortcuts", () => {
    const html = render([]);
    expect(html).toContain('aria-label="Send with Win"');
    expect(html).toContain("Windows shortcuts");
    expect(html).toContain('aria-label="Clipboard history: Win + V"');
    expect(html).toContain('aria-label="Copilot: Win + C"');
  });

  it("marks the key the binding already sends", () => {
    const html = render(["KeyQ"]);
    expect(html).toMatch(/class="kp-key on" data-code="KeyQ"/);
  });

  it("speaks Mac on a Mac: command caps, no Windows shortcuts", () => {
    const html = render([], true);
    expect(html).toContain('data-icon="command"');
    expect(html).not.toContain("Windows shortcuts");
  });
});

const app = (id: string, name: string, kind: AppEntry["kind"], launch: string): AppEntry => ({
  id,
  name,
  kind,
  launch,
  icon: null,
  source: kind === "steam" ? "steam" : "windows",
});
const catalog: AppEntry[] = [
  app("builtin:shell:AppsFolder\\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App", "Calculator", "aumid", "shell:AppsFolder\\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"),
  app("app:C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs\\Discord.lnk", "Discord", "path", "C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs\\Discord.lnk"),
  app("app:C:\\Program Files\\Tool\\tool.exe", "Tool", "path", "C:\\Program Files\\Tool\\tool.exe"),
  app("steam:620", "Portal 2", "steam", "steam://rungameid/620"),
];
const calc: Action = { type: "launch_app", app: { kind: "aumid", id: "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App" } };

const renderLaunch = (action: Action | undefined, open = true) =>
  renderToStaticMarkup(
    <KeyPicker value={[]} onPick={() => {}} mac={false} defaultOpen={open} action={action} catalog={catalog} onLaunch={() => {}} />,
  );

describe("KeyPicker launch tab", () => {
  it("opens on the program list when the key already launches one, marking it", () => {
    const html = renderLaunch(calc);
    expect(html).toContain('role="tab" aria-selected="true">Launch program</button>');
    expect(html).toContain('placeholder="Search 4 programs"');
    expect(html).toMatch(/aria-label="Launch Calculator" aria-pressed="true"/);
    expect(html).toMatch(/aria-label="Launch Discord" aria-pressed="false"/);
    for (const title of ["Windows apps", "Installed programs", "Steam games"]) expect(html).toContain(title);
  });

  it("lists shortcuts, exes, Store apps and games, one row each", () => {
    const html = renderLaunch(calc);
    const rows = [...html.matchAll(/class="kp-app" data-launch="([^"]+)"/g)].map((m) => m[1]);
    expect(rows.sort()).toEqual(catalog.map((e) => e.launch.replace(/&/g, "&amp;")).sort());
  });

  it("starts on the key list for a key binding, with both tabs offered", () => {
    const html = renderLaunch(undefined);
    expect(html).toContain('role="tab" aria-selected="true">Send a key</button>');
    expect(html).toContain('role="tab" aria-selected="false">Launch program</button>');
    expect(html).toContain('data-code="KeyA"');
  });

  it("names the program on its closed button", () => {
    const html = renderLaunch(calc, false);
    expect(html).toContain('aria-label="Launches Calculator. Choose a key or program"');
    expect(html).toContain("Open Calculator");
  });
});
