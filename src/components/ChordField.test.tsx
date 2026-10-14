import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import ChordField from "./ChordField";

describe("ChordField", () => {
  it("shows the chord in friendly form with a Record button", () => {
    const html = renderToStaticMarkup(<ChordField id="c" initial={["Meta", "KeyS"]} onChange={() => {}} />);
    expect(html).toContain('value="Win + S"');
    expect(html).toContain(">Record</button>");
  });

  it("locks both controls when disabled", () => {
    const html = renderToStaticMarkup(<ChordField id="c" initial={[]} disabled onChange={() => {}} />);
    expect(html.match(/disabled=""/g)).toHaveLength(2);
  });
});
