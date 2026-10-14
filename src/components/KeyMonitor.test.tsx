import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import KeyMonitor from "./KeyMonitor";

describe("KeyMonitor", () => {
  it("prompts before the first key", () => {
    expect(renderToStaticMarkup(<KeyMonitor last={null} />)).toContain("Press any key to see what the OS reports");
  });

  it("shows code, held modifiers and the raw OS report", () => {
    const html = renderToStaticMarkup(
      <KeyMonitor last={{ code: "F14", mods: ["Meta", "Shift"], down: true, raw: "scan 0x65 ext 0 vk 0x7D" }} />,
    );
    expect(html).toContain("last press · <b>F14</b> with Win + Shift");
    expect(html).toContain("scan 0x65 ext 0 vk 0x7D");
  });
});
