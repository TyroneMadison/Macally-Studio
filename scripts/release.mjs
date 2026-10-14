#!/usr/bin/env node
// Usage: node scripts/release.mjs X.Y
// Rewrites the version in package.json, src-tauri/tauri.conf.json,
// src-tauri/Cargo.toml (+ Cargo.lock) and the README badge, commits
// "Version X.Y" and tags vX.Y. Public versions are "Major.Minor" with Minor 0-9.
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { basename } from "node:path";
import { fileURLToPath } from "node:url";

/** Same rule as src-tauri/src/version.rs::is_valid_release: "X.Y", Y a single digit. */
export function isValidRelease(s) {
  return /^\d+\.\d$/.test(s);
}

/** Returns `text` with the version rewritten for the given file name. */
export function bump(file, text, xy) {
  const semver = `${xy}.0`;
  switch (basename(file)) {
    case "package.json":
    case "tauri.conf.json":
      return text.replace(/^(\s*"version":\s*")[^"]*(")/m, `$1${semver}$2`);
    case "Cargo.toml":
      return text.replace(/^(version\s*=\s*")[^"]*(")/m, `$1${semver}$2`);
    case "README.md":
      return text.replace(/Version-\d+\.\d-/g, `Version-${xy}-`);
    default:
      throw new Error(`bump: unknown file ${file}`);
  }
}

const FILES = ["package.json", "src-tauri/tauri.conf.json", "src-tauri/Cargo.toml", "README.md"];

function main(xy) {
  if (!isValidRelease(xy)) {
    console.error(`usage: node scripts/release.mjs X.Y   (Y is 0-9; got "${xy ?? ""}")`);
    process.exit(1);
  }
  for (const f of FILES) {
    const before = readFileSync(f, "utf8");
    const after = bump(f, before, xy);
    if (after === before) {
      console.error(`${f}: no version line found`);
      process.exit(1);
    }
    writeFileSync(f, after);
  }
  execFileSync("cargo", ["update", "--workspace", "--manifest-path", "src-tauri/Cargo.toml"], { stdio: "inherit" });
  execFileSync("git", ["add", ...FILES, "src-tauri/Cargo.lock"], { stdio: "inherit" });
  execFileSync("git", ["commit", "-m", `Version ${xy}`], { stdio: "inherit" });
  execFileSync("git", ["tag", `v${xy}`], { stdio: "inherit" });
  console.log(`Version ${xy} committed and tagged v${xy}. Push with: git push && git push origin v${xy}`);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  main(process.argv[2]);
}
