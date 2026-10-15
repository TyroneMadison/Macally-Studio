import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import Rail from "./Rail";

describe("Rail", () => {
  it("has no play/pause: remapping is always on", () => {
    const html = renderToStaticMarkup(<Rail view="keyboard" onNav={() => {}} />);
    expect(html).not.toMatch(/pause|resume/i);
    expect(html).toContain('aria-label="Keyboard"');
  });
});
