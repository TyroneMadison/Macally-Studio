# Community profiles

Profiles made by Macally Studio users. Each `.json` file in this folder is one profile.

## Use a profile

1. Open the file on GitHub and click **Download raw file**.
2. In Macally Studio, go to **Profiles → Import** and pick the file.
3. Activate it, or give it a hotkey or auto-activate apps.

## Share yours

1. Build and test the profile in Macally Studio.
2. Export it with **Profiles → Export**.
3. Name it `<your-github-name>-<what-it-is>.json`, for example `tyronemadison-photoshop.json`, and add it to this folder in a pull request.
4. In the pull request, say what the profile is for and which system you use it on.

Before you send it:

- `"builtin"` is `false`, and `"id"` is lowercase words joined by dashes, unique in this folder.
- Clear `"hotkey"`, and keep `"autoApps"` only when the rule helps everyone (for example `"Photoshop.exe"`).
- Prefer actions that work on any computer: key combos, websites, Steam games. An app launched by a path on your own disk will not exist on someone else's.

## Format

[example-profile.json](example-profile.json) shows every field.

| Field | Meaning |
| --- | --- |
| `schema` | Always `1`. |
| `id` | File-safe id, for example `photo-editing`. |
| `name` | Name shown in the app. |
| `builtin` | Always `false` for shared profiles. |
| `hotkey` | Shortcut that activates the profile, for example `"Ctrl+Alt+2"`, or `null`. |
| `autoApps` | Apps that activate the profile when they come to the front: exe name on Windows (`"Photoshop.exe"`), bundle id on macOS (`"com.adobe.Photoshop"`). |
| `bindings` | Key name to `{ "mods": [], "action": {} }`. `mods` lists modifiers that must be held (Export writes them for you); `[]` means a plain press. |

Key names are the standard web key codes: `KeyA`, `Digit1`, `F13`, `Eject`, `NumpadEqual`, `MetaLeft` (command), `AltLeft` (option). The Key Monitor in the app shows the name of any key you press.

| Action `type` | Fields | Example |
| --- | --- | --- |
| `key_combo` | `keys` | `{ "type": "key_combo", "keys": ["ControlLeft", "KeyZ"] }` |
| `launch_app` | `app` | `{ "type": "launch_app", "app": { "kind": "bundle", "id": "com.apple.calculator" } }` |
| `steam_game` | `appid`, `name` | `{ "type": "steam_game", "appid": 620, "name": "Portal 2" }` |
| `open` | `target` | `{ "type": "open", "target": "https://github.com" }` |
| `command` | `program`, `args`, `hidden` | `{ "type": "command", "program": "powershell", "args": ["-File", "C:\\scripts\\x.ps1"], "hidden": true }` |
| `switch_profile` | `profile` | `{ "type": "switch_profile", "profile": "gaming" }` |
| `pass_through` | none | `{ "type": "pass_through" }` |
| `block` | none | `{ "type": "block" }` (the key does nothing) |

`launch_app` kinds: `aumid` with `id` (Windows Start-menu and Store apps), `path` with `path` and `args`, `bundle` with `id` (macOS).
