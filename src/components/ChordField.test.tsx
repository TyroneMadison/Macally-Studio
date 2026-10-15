import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import ChordField from "./ChordField";

describe("ChordField", () => {
  it("shows the chord in friendly form, typed rather than recorded", () => {
    const html = renderToStaticMarkup(<ChordField id="c" initial={["Meta", "KeyS"]} onChange={() => {}} />);
    expect(html).toContain('value="Win + S"');
    expect(html).not.toContain("Record");
    expect(html).not.toContain("<button");
  });

  it("locks the field when disabled", () => {
    const html = renderToStaticMarkup(<ChordField id="c" initial={[]} disabled onChange={() => {}} />);
    expect(html.match(/disabled=""/g)).toHaveLength(1);
  });
});
