# 📜 Changelog

[![Latest release](https://img.shields.io/github/v/release/TyroneMadison/Macally-Studio?label=latest%20release&color=4C8DFF&style=for-the-badge)](https://github.com/TyroneMadison/Macally-Studio/releases/latest) [![Commits](https://img.shields.io/github/commit-activity/t/TyroneMadison/Macally-Studio?label=commits&color=8E8E97&style=flat-square)](https://github.com/TyroneMadison/Macally-Studio/commits/main) ![Versions](https://img.shields.io/badge/versions-1.0%20%E2%80%A6%201.9%20%E2%86%92%202.0-3DD68C?style=flat-square)

Every entry below is a commit from this repository's history. Versions go 1.0, 1.1 … 1.9, then 2.0, and each `vX.Y` tag publishes a GitHub Release with the Windows and macOS installers.

Legend: ✨ new · 🐛 fix · 🔥 removed · 🎨 look and feel · 📝 docs · 🔖 version bump

## 🧪 Unreleased · Fn key

![unreleased](https://img.shields.io/badge/unreleased-main-F5A524?style=for-the-badge)

The Fn key on this Macally is handled inside the keyboard’s firmware and never reaches the computer — a HID probe confirmed the keyboard reports only standard keyboard, consumer and system-control usages, with no vendor collection, so no app can see Fn. The app now says this plainly instead of letting you map Fn to nothing.

- ✨ Selecting Fn explains it plainly and offers what works instead: jump to Left Control, Caps Lock or Left Command and give it any action in the Default or a game profile, or make Caps Lock send Ctrl in one click. It also explains fn + esc (Fn-Lock), which makes the top row send F1–F15 that can be remapped. In the key inspector and the full-screen game editor
- 🔥 Fn is marked firmware-only on the keyboard and is no longer offered as a remappable source
- ✨ One-click “Reset Fn key” clears any leftover Fn binding from every profile (game profiles included) and rebuilds the engine, with a confirmation toast; old Fn bindings are also cleared automatically on launch
- 🐛 The nav-cluster key printed fn, likely a second fn key handled by the keyboard, now says so and points to the Key Monitor to check it, instead of quietly taking a remap that may never fire

## 🎮 v1.4 · Gaming profiles

[![v1.4](https://img.shields.io/badge/v1.4-2026--10--15-4C8DFF?style=for-the-badge)](https://github.com/TyroneMadison/Macally-Studio/releases/tag/v1.4) ![commits](https://img.shields.io/badge/commits-14-8E8E97?style=flat-square)

Every game gets its own keys, which switch in while the game is in front and back when you leave it. The 3D keyboard no longer jitters while it turns, a key is remapped by clicking, and play/pause is gone (remapping is always on).

- ✨ Game profiles in the store: each game gets its own profile, with the game's exe or install folder, icon and auto-switch flag [`3cf8937`](https://github.com/TyroneMadison/Macally-Studio/commit/3cf8937)
- ✨ Tell which game an app starts: the exe of a program, shortcut or Start-menu app, or a Steam game's install folder; share the icon cache [`31e70be`](https://github.com/TyroneMadison/Macally-Studio/commit/31e70be)
- ✨ Switch to a game's profile while the game is in front and back when it loses focus or quits, debounced; commands and a profile-switched event for the coming games UI [`c0dc232`](https://github.com/TyroneMadison/Macally-Studio/commit/c0dc232)
- 🐛 Fix the 3D keyboard jittering while it is rotated: the camera's own moves no longer come back to it as requests [`1c7e2ec`](https://github.com/TyroneMadison/Macally-Studio/commit/1c7e2ec)
- 🔥 Remap by clicking only: remove Press a key, Pick key by pressing and the engine's Record mode [`21c4d5a`](https://github.com/TyroneMadison/Macally-Studio/commit/21c4d5a)
- 🔥 Remove play/pause and the pause function everywhere: remapping is always on [`c0b5fdf`](https://github.com/TyroneMadison/Macally-Studio/commit/c0b5fdf)
- ✨ Game profiles run on top of the Default profile, a key can be turned off, and a game knows its exe path [`8d37aae`](https://github.com/TyroneMadison/Macally-Studio/commit/8d37aae)
- ✨ Game profiles UI: the game status in the rail corner, the detection notice, the Games panel and Add a game, after the mockups in docs/mockups [`7d8a24e`](https://github.com/TyroneMadison/Macally-Studio/commit/7d8a24e)
- ✨ Full-screen profile editor: opens when a game's profile switches in, detected or picked by hand (04c) [`88489ba`](https://github.com/TyroneMadison/Macally-Studio/commit/88489ba)
- 🎨 Editor and Game profiles polish: short Mac legends (cmd, opt, ctrl, pg up) on the flat keyboard, and a long exe path wraps beside the game instead of pushing Duplicate and Delete down [`d8e0bbb`](https://github.com/TyroneMadison/Macally-Studio/commit/d8e0bbb)
- 📝 README, changelog and real screenshots [`eded7f7`](https://github.com/TyroneMadison/Macally-Studio/commit/eded7f7) [`4dcf5b4`](https://github.com/TyroneMadison/Macally-Studio/commit/4dcf5b4) [`278bc48`](https://github.com/TyroneMadison/Macally-Studio/commit/278bc48)
- 🔖 Version 1.4 · [compare v1.3…v1.4](https://github.com/TyroneMadison/Macally-Studio/compare/v1.3...v1.4)

<table><tr>
<td><img src="docs/images/game-detected.png" alt="Notepad as a stand-in game: detected, its profile switched in and its editor opened"></td>
<td><img src="docs/images/game-profiles.png" alt="Game profiles: the Default and a game with its own keys and auto-switch"></td>
</tr></table>

## 🚀 v1.3 · Launch program

[![v1.3](https://img.shields.io/badge/v1.3-2026--10--14-4C8DFF?style=for-the-badge)](https://github.com/TyroneMadison/Macally-Studio/releases/tag/v1.3) ![commits](https://img.shields.io/badge/commits-2-8E8E97?style=flat-square)

- ✨ Launch program in the key picker: installed programs from Start-menu shortcuts, App Paths and the Uninstall registry, with icons and search; programs start in their own folder [`0050236`](https://github.com/TyroneMadison/Macally-Studio/commit/0050236)
- 🔖 Version 1.3 [`e9d14b6`](https://github.com/TyroneMadison/Macally-Studio/commit/e9d14b6) · [compare v1.2…v1.3](https://github.com/TyroneMadison/Macally-Studio/compare/v1.2...v1.3)

<p align="center"><img src="docs/images/launch-program.png" alt="Launch program tab in the key picker with real app icons" width="760"></p>

## 🪟 v1.2 · The Windows key and the keycap picker

[![v1.2](https://img.shields.io/badge/v1.2-2026--10--14-4C8DFF?style=for-the-badge)](https://github.com/TyroneMadison/Macally-Studio/releases/tag/v1.2) ![commits](https://img.shields.io/badge/commits-3-8E8E97?style=flat-square)

- 🐛 Fix the dead Windows key: Command is Win again, keys remapped to Win work in any chord, and swallowed presses no longer pop Start [`aa7d35c`](https://github.com/TyroneMadison/Macally-Studio/commit/aa7d35c)
- ✨ Keycap picker: every key as a thumbnail, Windows logo on Win, send-with modifiers and Windows shortcuts like Win+V [`38ce4dd`](https://github.com/TyroneMadison/Macally-Studio/commit/38ce4dd)
- 🔖 Version 1.2 [`9aa366a`](https://github.com/TyroneMadison/Macally-Studio/commit/9aa366a) · [compare v1.1…v1.2](https://github.com/TyroneMadison/Macally-Studio/compare/v1.1...v1.2)

<table><tr>
<td><img src="docs/images/key-picker-win.png" alt="Keycap picker with the Win key and its Windows logo"></td>
<td><img src="docs/images/win-shortcuts.png" alt="Windows shortcut presets in the key picker: Win + V, Win + C and more"></td>
</tr></table>

## 🌀 v1.1 · 360° view, lighting and easy remapping

[![v1.1](https://img.shields.io/badge/v1.1-2026--10--14-4C8DFF?style=for-the-badge)](https://github.com/TyroneMadison/Macally-Studio/releases/tag/v1.1) ![commits](https://img.shields.io/badge/commits-5-8E8E97?style=flat-square)

- ✨ 3D view: full 360 orbit with tilt, brighter studio lighting, adjustable brightness and backlight [`85c1644`](https://github.com/TyroneMadison/Macally-Studio/commit/85c1644)
- ✨ Remap keys by pressing or choosing from a list, pick keys by pressing them, and edit built-in profiles in place [`a3fa956`](https://github.com/TyroneMadison/Macally-Studio/commit/a3fa956)
- 🐛 Remap fixes: one recording at a time, edits end a recording, saves build on the latest binding, hide keys the OS cannot send, zoom to 250% [`2b0eab2`](https://github.com/TyroneMadison/Macally-Studio/commit/2b0eab2)
- 🎨 Start the 3D view closer and refresh the keyboard screenshot [`c6e1853`](https://github.com/TyroneMadison/Macally-Studio/commit/c6e1853)
- 🔖 Version 1.1 [`bcba892`](https://github.com/TyroneMadison/Macally-Studio/commit/bcba892) · [compare v1.0…v1.1](https://github.com/TyroneMadison/Macally-Studio/compare/v1.0...v1.1)

<p align="center"><img src="docs/images/keyboard-view.png" alt="Keyboard view with the 3D keyboard, rotate, tilt, zoom and lighting controls, and the key inspector" width="760"></p>

## 🎉 v1.0 · First release, and everything before it

[![v1.0](https://img.shields.io/badge/v1.0-2026--10--14-4C8DFF?style=for-the-badge)](https://github.com/TyroneMadison/Macally-Studio/releases/tag/v1.0) ![commits](https://img.shields.io/badge/commits-57-8E8E97?style=flat-square) ![since](https://img.shields.io/badge/first%20commit-2026--10--14-8E8E97?style=flat-square)

The whole app came together in the commits up to the `v1.0` tag, from the first scaffold to the first public installers.

**🧱 Foundation**

- Scaffold Tauri app shell, theme, fonts, version policy, release script and CI [`f4078f0`](https://github.com/TyroneMadison/Macally-Studio/commit/f4078f0)
- Add 107-key layout, keymap, get_layout command and the 3D keyboard view [`1668c19`](https://github.com/TyroneMadison/Macally-Studio/commit/1668c19)
- Add Blender keyboard generator and README hero render [`499f266`](https://github.com/TyroneMadison/Macally-Studio/commit/499f266)
- Tidy gitignore [`94afd87`](https://github.com/TyroneMadison/Macally-Studio/commit/94afd87)

**⌨️ Remap engine**

- Add action types [`c7185b7`](https://github.com/TyroneMadison/Macally-Studio/commit/c7185b7)
- Add the remap engine core [`6d865df`](https://github.com/TyroneMadison/Macally-Studio/commit/6d865df)
- Add the Windows keyboard hook and hook self-test [`233a8a3`](https://github.com/TyroneMadison/Macally-Studio/commit/233a8a3)
- Add the macOS event tap [`db051d2`](https://github.com/TyroneMadison/Macally-Studio/commit/db051d2)
- Run key actions and start the engine [`7a51b42`](https://github.com/TyroneMadison/Macally-Studio/commit/7a51b42)
- Keep held-with modifiers out of injected chords on Windows [`6810624`](https://github.com/TyroneMadison/Macally-Studio/commit/6810624)
- Give macOS key events the flags of remapped modifiers so Cmd remapped to Ctrl makes Cmd+C arrive as Ctrl+C [`2e1b09d`](https://github.com/TyroneMadison/Macally-Studio/commit/2e1b09d)
- Treat each macOS Caps Lock event as a full key press [`ba2ba6a`](https://github.com/TyroneMadison/Macally-Studio/commit/ba2ba6a)

**🗂️ Profiles and settings**

- Add profile and settings store [`6aecf74`](https://github.com/TyroneMadison/Macally-Studio/commit/6aecf74)
- Ship Windows Default and macOS Default profiles [`da35ef2`](https://github.com/TyroneMadison/Macally-Studio/commit/da35ef2)
- Add profile and settings commands [`854c8a5`](https://github.com/TyroneMadison/Macally-Studio/commit/854c8a5)
- Add typed IPC wrappers and app state [`e3b48a2`](https://github.com/TyroneMadison/Macally-Studio/commit/e3b48a2)
- Add hotkey formatting and profile helpers [`3af215d`](https://github.com/TyroneMadison/Macally-Studio/commit/3af215d)
- Add Profiles view [`016c76b`](https://github.com/TyroneMadison/Macally-Studio/commit/016c76b)
- Add Settings view [`cdc5177`](https://github.com/TyroneMadison/Macally-Studio/commit/cdc5177)
- Note that every profile switch goes through AppState::activate [`3406f53`](https://github.com/TyroneMadison/Macally-Studio/commit/3406f53)
- Drop clashing hotkeys on import and back up unreadable settings.json [`4608508`](https://github.com/TyroneMadison/Macally-Studio/commit/4608508)
- Queue saves and send only the changed fields so quick edits do not undo each other [`b9c2054`](https://github.com/TyroneMadison/Macally-Studio/commit/b9c2054)
- Show the Fn-Lock hint, keyboard key count and permissions note [`9c920ec`](https://github.com/TyroneMadison/Macally-Studio/commit/9c920ec)
- Add the foreground watcher and per-app profile switching [`41e8629`](https://github.com/TyroneMadison/Macally-Studio/commit/41e8629)
- Add global hotkeys [`cbcf57b`](https://github.com/TyroneMadison/Macally-Studio/commit/cbcf57b)
- Check the community example profile in a Rust test [`84381c6`](https://github.com/TyroneMadison/Macally-Studio/commit/84381c6)

**🖥️ App, tray and permissions**

- Add the tray menu [`fc45809`](https://github.com/TyroneMadison/Macally-Studio/commit/fc45809)
- Wire the engine, tray, hotkeys, foreground watcher and macOS permissions screen into the app [`860a358`](https://github.com/TyroneMadison/Macally-Studio/commit/860a358)
- Ask for elevation once per run and apply login settings only at startup and when they change [`a4986f8`](https://github.com/TyroneMadison/Macally-Studio/commit/a4986f8)
- Run get_status off the main thread [`d93e539`](https://github.com/TyroneMadison/Macally-Studio/commit/d93e539)
- Start the window hidden so a login start never flashes it, and reopen it from the Dock on macOS [`5a125cd`](https://github.com/TyroneMadison/Macally-Studio/commit/5a125cd)

**🧭 App catalog and action picker**

- Add app catalog entry type, icon file names, ordering and dedupe [`bcee5a6`](https://github.com/TyroneMadison/Macally-Studio/commit/bcee5a6)
- Add Steam library scanner with libraryfolders and appmanifest parsing [`80ba73d`](https://github.com/TyroneMadison/Macally-Studio/commit/80ba73d)
- Add Windows Start apps scanner with shell icon extraction [`e5df627`](https://github.com/TyroneMadison/Macally-Studio/commit/e5df627)
- Add macOS app bundle scanner with NSWorkspace icons [`e6679ec`](https://github.com/TyroneMadison/Macally-Studio/commit/e6679ec)
- Cache the app catalog and add get_catalog and refresh_catalog commands [`2bcb149`](https://github.com/TyroneMadison/Macally-Studio/commit/2bcb149)
- Add chord, record and catalog helpers for the action picker [`cb926a5`](https://github.com/TyroneMadison/Macally-Studio/commit/cb926a5)
- Add chord field with Record for key combo bindings [`ae1cad9`](https://github.com/TyroneMadison/Macally-Studio/commit/ae1cad9)
- Add action picker with app, Steam, URL, command and profile tabs [`204a421`](https://github.com/TyroneMadison/Macally-Studio/commit/204a421)
- Add key inspector and key monitor strip [`fc21da7`](https://github.com/TyroneMadison/Macally-Studio/commit/fc21da7)
- Wire the inspector, action picker and key monitor into the Keyboard view [`33d6ab5`](https://github.com/TyroneMadison/Macally-Studio/commit/33d6ab5)
- Keep Monitor mode after a Record ends in an unmounted chord field [`93bb87d`](https://github.com/TyroneMadison/Macally-Studio/commit/93bb87d)
- Keep Key Monitor off while the window is hidden [`545e00c`](https://github.com/TyroneMadison/Macally-Studio/commit/545e00c)
- Keep browsed programs in the catalog and send the last catalog progress after the cache is saved [`f5bbfb5`](https://github.com/TyroneMadison/Macally-Studio/commit/f5bbfb5)
- Always scan the main Steam library [`6f1de1e`](https://github.com/TyroneMadison/Macally-Studio/commit/6f1de1e)
- Show modifiers in OS order and names and reject chords with unknown keys [`45a7ef7`](https://github.com/TyroneMadison/Macally-Studio/commit/45a7ef7)
- Use as_chunks_mut for the icon pixel swap [`d3127c6`](https://github.com/TyroneMadison/Macally-Studio/commit/d3127c6)

**🔄 Updates and releases**

- Set the updater public key [`e8aac19`](https://github.com/TyroneMadison/Macally-Studio/commit/e8aac19)
- Add release workflow, README, contributing guide, community profiles and issue forms [`24466fd`](https://github.com/TyroneMadison/Macally-Studio/commit/24466fd)
- Add update check and install commands [`dfc0f24`](https://github.com/TyroneMadison/Macally-Studio/commit/dfc0f24)
- Open Settings when an update is found [`74e0681`](https://github.com/TyroneMadison/Macally-Studio/commit/74e0681)
- Check for updates from the UI once it listens and fail install when no update exists [`08fe4df`](https://github.com/TyroneMadison/Macally-Studio/commit/08fe4df)

**🎨 Look and docs**

- Render bound keys as backlit caps with glowing legends [`4e32705`](https://github.com/TyroneMadison/Macally-Studio/commit/4e32705)
- Readable key names in the bindings grid and README screenshots [`fc777a3`](https://github.com/TyroneMadison/Macally-Studio/commit/fc777a3)
- Describe the macOS permissions screen in the README [`7d1d945`](https://github.com/TyroneMadison/Macally-Studio/commit/7d1d945)
- Explain re-granting macOS permissions after an update [`e77a13c`](https://github.com/TyroneMadison/Macally-Studio/commit/e77a13c)
- README: mention Fn-Lock in the Windows install steps [`3962f45`](https://github.com/TyroneMadison/Macally-Studio/commit/3962f45)

<table><tr>
<td><img src="docs/images/profiles.png" alt="Profiles view"></td>
<td><img src="docs/images/settings.png" alt="Settings and updates"></td>
</tr></table>

<sub>Profiles and Settings as they look in Version 1.3.</sub>
