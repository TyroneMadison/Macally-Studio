import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import KeyPicker from "./KeyPicker";
import { KEY_GROUPS } from "../lib/actions";

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
