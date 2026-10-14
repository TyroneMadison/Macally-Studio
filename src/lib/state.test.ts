import { describe, expect, it } from "vitest";

import { serial } from "./state";

describe("serial", () => {
  it("runs tasks one at a time and keeps going after a failure", async () => {
    const run = serial();
    const log: string[] = [];
    const task = (name: string, fail = false) => async () => {
      log.push(`${name} start`);
      await new Promise((r) => setTimeout(r, 5));
      log.push(`${name} end`);
      if (fail) throw new Error(name);
    };
    const a = run(task("a", true));
    const b = run(task("b"));
    await expect(a).rejects.toThrow("a");
    await b;
    expect(log).toEqual(["a start", "a end", "b start", "b end"]);
  });
});
