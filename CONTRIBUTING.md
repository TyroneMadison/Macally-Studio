# Contributing to Macally Studio

Thanks for helping. Bug reports, ideas, shared profiles and code are all welcome.

## Report a bug or suggest a feature

Use the issue forms: [Bug report](https://github.com/TyroneMadison/Macally-Studio/issues/new?template=bug_report.yml) or [Feature request](https://github.com/TyroneMadison/Macally-Studio/issues/new?template=feature_request.yml).

For a key that does not behave, include what the Key Monitor shows when you press it (Keyboard view, bottom of the inspector).

## Share a profile

See [community/README.md](community/README.md).

## Set up the project

You need Node.js 22.12 or newer, Rust 1.90 or newer, and the build tools for your system: Microsoft C++ Build Tools ("Desktop development with C++") on Windows, Xcode Command Line Tools (`xcode-select --install`) on macOS.

```sh
git clone https://github.com/TyroneMadison/Macally-Studio.git
cd Macally-Studio
npm install
npm run tauri dev
```

## Before you open a pull request

Run the same checks CI runs:

```sh
npm test
npm run build
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --features selftest -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Keep a pull request to one change, use a short plain commit subject ("Add Steam icon cache"), and say how you tested it and on which system. Changes to the remap engine (`src-tauri/src/engine/`) need a test on a real keyboard: say what you pressed and what happened.

## Where things live

| Path | What |
| --- | --- |
| `src-tauri/src/engine/` | keyboard hook (Windows) and event tap (macOS) |
| `src-tauri/src/keymap.rs` | key names and their Windows and macOS codes |
| `src-tauri/src/actions.rs` | what a bound key does |
| `src-tauri/src/profiles.rs` | profile and settings files |
| `src-tauri/src/catalog/` | app and Steam game lists |
| `src/components/Keyboard3D.tsx` | the 3D keyboard |
| `src/lib/layout.json` | position and size of all 107 keys |

## Releasing (maintainer)

Versions are Major.Minor with Minor from 0 to 9: 1.0, 1.1 and so on up to 1.9, then 2.0. GitHub Actions (`.github/workflows/release.yml`) builds the installers when a tag `vX.Y` is pushed and publishes the release "Version X.Y".

### One-time setup: update signing key

Updates are signed, and the private key never goes into the repository.

1. Generate the keypair. It asks for a password: choose a non-empty one and keep it in your password manager.

   ```sh
   npx tauri signer generate -w "$HOME/.tauri/macally-studio.key"
   ```

   This writes the private key `~/.tauri/macally-studio.key` and the public key `~/.tauri/macally-studio.key.pub`. Back up the private key: without it, installed copies can never receive another update.

2. Put the contents of `macally-studio.key.pub` (the text, not the path) into `src-tauri/tauri.conf.json` at `plugins.updater.pubkey`, and commit.

3. Add the two repository secrets from Git Bash or a macOS terminal:

   ```sh
   gh secret set TAURI_SIGNING_PRIVATE_KEY --repo TyroneMadison/Macally-Studio --body "$(cat "$HOME/.tauri/macally-studio.key")"
   gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --repo TyroneMadison/Macally-Studio
   ```

   The second command prompts for the password from step 1. Check with `gh secret list --repo TyroneMadison/Macally-Studio`.

### Each release

```sh
node scripts/release.mjs 1.1
git push origin main v1.1
```

`release.mjs` refuses a minor above 9. It writes the version into `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and the README badge, commits "Version 1.1" and tags `v1.1`. The push starts the release workflow. It builds Windows (`-setup.exe`, `.msi`) and macOS (universal `.dmg`), publishes the GitHub Release "Version 1.1", and uploads the `latest.json` that the in-app updater reads.
