import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import Rail from "./Rail";

describe("Rail", () => {
  it("has no play/pause: remapping is always on", () => {
    const html = renderToStaticMarkup(<Rail view="keyboard" onNav={() => {}} />);
    expect(html).not.toMatch(/pause|resume/i);
    expect(html).toContain('aria-label="Keyboard"');
  });

  it("offers Game profiles and puts the game status in the bottom corner", () => {
    const html = renderToStaticMarkup(<Rail view="games" onNav={() => {}} corner={<b>corner</b>} />);
    expect(html).toContain('aria-label="Game profiles" aria-current="page"');
    expect(html).toMatch(/rail-spacer"><\/div><b>corner<\/b><\/nav>$/);
  });
});
