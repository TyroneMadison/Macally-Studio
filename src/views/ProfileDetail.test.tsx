import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import ProfileDetail from "./ProfileDetail";
import type { AppState } from "../lib/state";
import type { Profile, Settings } from "../lib/types";

const profile = (builtin: boolean): Profile => ({
  schema: 1,
  id: builtin ? "windows-default" : "mine",
  name: builtin ? "Windows Default" : "Mine",
  builtin,
  hotkey: null,
  autoApps: ["Photoshop.exe"],
  bindings: {},
});

const render = (p: Profile) =>
  renderToStaticMarkup(
    <ProfileDetail
      store={{ layout: { unit: 19.05, keys: [] }, catalog: [], profiles: [p], settings: {} as Settings, activeProfile: null } as unknown as AppState}
      profile={p}
      onSelect={() => {}}
      onEditKeys={() => {}}
    />,
  );

describe("ProfileDetail", () => {
  it("edits built-ins in place and offers Restore defaults instead of Delete", () => {
    const html = render(profile(true));
    expect(html).not.toContain('disabled=""');
    expect(html).toContain("+ Add app rule");
    expect(html).toContain('aria-label="Remove Photoshop.exe"');
    expect(html).toContain(">Restore defaults</button>");
    expect(html).not.toContain(">Delete</button>");
  });

  it("lets user profiles be deleted, not restored", () => {
    const html = render(profile(false));
    expect(html).toContain(">Delete</button>");
    expect(html).not.toContain("Restore defaults");
  });
});
