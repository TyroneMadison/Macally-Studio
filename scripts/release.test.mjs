import { test } from "node:test";
import assert from "node:assert/strict";
import { bump, isValidRelease } from "./release.mjs";

test("isValidRelease accepts X.Y with a single-digit minor", () => {
  assert.equal(isValidRelease("1.0"), true);
  assert.equal(isValidRelease("1.9"), true);
  assert.equal(isValidRelease("12.3"), true);
  assert.equal(isValidRelease("1.10"), false);
  assert.equal(isValidRelease("1.0.1"), false);
  assert.equal(isValidRelease("v1.0"), false);
  assert.equal(isValidRelease(""), false);
});

test("bump rewrites only the version line of each file", () => {
  assert.equal(
    bump("package.json", '{\n  "name": "macally-studio",\n  "version": "1.0.0",\n  "type": "module"\n}\n', "1.1"),
    '{\n  "name": "macally-studio",\n  "version": "1.1.0",\n  "type": "module"\n}\n',
  );
  assert.equal(
    bump("src-tauri/Cargo.toml", '[package]\nname = "macally-studio"\nversion = "1.0.0"\n\n[dependencies]\nserde = { version = "1" }\n', "2.0"),
    '[package]\nname = "macally-studio"\nversion = "2.0.0"\n\n[dependencies]\nserde = { version = "1" }\n',
  );
  assert.equal(
    bump("README.md", "![Version](https://img.shields.io/badge/Version-1.0-4C8DFF)\n", "1.3"),
    "![Version](https://img.shields.io/badge/Version-1.3-4C8DFF)\n",
  );
  assert.throws(() => bump("other.txt", "", "1.0"), /unknown file/);
});
