import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import GameStatus from "./GameStatus";
import GameToast from "./GameToast";
import type { Game } from "../lib/types";

const valorant: Game = { name: "Valorant", exe: "VALORANT.exe", autoSwitch: true };
const noop = () => {};

describe("GameStatus (bottom-left corner)", () => {
  it("watches for games while no game profile is active", () => {
    const html = renderToStaticMarkup(<GameStatus game={null} watching live={false} auto={false} onOpen={noop} />);
    expect(html).toContain("gs-idle");
    expect(html).toContain("Watching for games");
    expect(html).not.toContain("AUTO");
  });

  it("says when game detection is off", () => {
    const html = renderToStaticMarkup(<GameStatus game={null} watching={false} live={false} auto={false} onOpen={noop} />);
    expect(html).toContain("Game detection off");
  });

  it("shows the detected game with its live dot and AUTO", () => {
    const html = renderToStaticMarkup(<GameStatus game={valorant} watching live auto pop={7} onOpen={noop} />);
    expect(html).toContain('aria-label="Valorant is in front, its profile is active. Open game profiles"');
    expect(html).toContain('class="gs-name">Valorant</span>');
    expect(html).toContain("gs-live");
    expect(html).toContain('class="gs-pill">AUTO</span>');
    expect(html).toContain("tile-letter");
  });

  it("shows MANUAL without the live dot for a game profile picked by hand", () => {
    const html = renderToStaticMarkup(<GameStatus game={{ ...valorant, autoSwitch: false }} watching live={false} auto={false} onOpen={noop} />);
    expect(html).toContain('class="gs-pill manual">MANUAL</span>');
    expect(html).not.toContain("gs-live");
  });
});

describe("GameToast", () => {
  it("announces the game and the profile it switched to", () => {
    const html = renderToStaticMarkup(<GameToast game={valorant} profileName="Valorant" bindings={14} auto onDone={noop} />);
    expect(html).toContain('role="status"');
    expect(html).toContain("Valorant detected");
    expect(html).toContain("Switched to <b>Valorant</b> profile · 14 bindings");
    expect(html).toContain(">AUTO<");
  });
});
