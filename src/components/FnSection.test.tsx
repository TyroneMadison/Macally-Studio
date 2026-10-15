import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import FnSection from "./FnSection";
import type { Profile } from "../lib/types";

const profile = (bindings: Profile["bindings"] = {}): Profile => ({
  schema: 1,
  id: "windows-default",
  name: "Windows Default",
  builtin: true,
  hotkey: null,
  autoApps: [],
  bindings,
});

const noop = async () => {};
const render = (p = profile()) =>
  renderToStaticMarkup(<FnSection profile={p} saveBinding={noop} resetKeyEverywhere={async () => 0} />);

describe("FnSection", () => {
  it("explains honestly that Fn cannot be remapped", () => {
    const html = render();
    expect(html).toContain("Fn can’t be remapped");
    expect(html).toContain("handled inside the keyboard");
    expect(html).toContain("never reaches");
    // no remap picker is offered for Fn
    expect(html).not.toContain("Remap to");
  });

  it("offers the Caps Lock workaround when Caps is not already Ctrl", () => {
    const html = render();
    expect(html).toContain("Use Caps Lock as Ctrl");
    expect(html).toContain("bottom-left");
  });

  it("shows Caps Lock is already Ctrl instead of offering it again", () => {
    const html = render(profile({ CapsLock: { mods: [], action: { type: "key_combo", keys: ["ControlLeft"] } } }));
    expect(html).toContain("Caps Lock already sends Ctrl");
    expect(html).not.toContain("Use Caps Lock as Ctrl");
  });

  it("always offers Reset Fn key", () => {
    expect(render()).toContain("Reset Fn key");
  });

  it("says what fn still does: fn+esc Fn-Lock makes the top row remappable F-keys", () => {
    const html = render();
    expect(html).toContain("fn + esc");
    expect(html).toContain("F1–F15");
  });

  it("jumps to a nearby key to remap it to anything when the view allows it", () => {
    const html = renderToStaticMarkup(
      <FnSection profile={profile()} saveBinding={noop} resetKeyEverywhere={async () => 0} onSelectKey={() => {}} />,
    );
    expect(html).toContain("Remap a nearby key instead");
    for (const code of ["CapsLock", "ControlLeft", "MetaLeft"]) expect(html).toContain(`data-code="${code}"`);
    // without a way to select keys, no dead buttons
    expect(render()).not.toContain("Remap a nearby key instead");
  });
});
