/** One physical key. Units are key units (1u = Layout.unit mm); row 0 is the F row. */
export type LayoutKey = {
  code: string; // W3C KeyboardEvent.code, e.g. "KeyA", "NumpadEnter"
  row: number;
  x: number;
  w: number;
  h: number;
  mac: string; // legend printed on the Macally
  win: string; // what Windows calls the key
  verify?: boolean; // OS code not yet confirmed on hardware
};

export type Layout = { unit: number; keys: LayoutKey[] };

// ---- Mirrors of the Rust types (src-tauri/src/{actions,profiles}.rs), camelCase on the wire ----

export type Modifier = "Ctrl" | "Alt" | "Shift" | "Meta";

export type AppRef =
  | { kind: "aumid"; id: string }
  | { kind: "path"; path: string; args: string }
  | { kind: "bundle"; id: string };

export type Action =
  | { type: "key_combo"; keys: string[] }
  | { type: "launch_app"; app: AppRef }
  | { type: "steam_game"; appid: number; name: string }
  | { type: "open"; target: string }
  | { type: "command"; program: string; args: string[]; hidden: boolean }
  | { type: "switch_profile"; profile: string }
  | { type: "pass_through" }
  | { type: "block" }; // the key does nothing at all

export type Binding = { mods: Modifier[]; action: Action };

export type Profile = {
  schema: number;
  id: string;
  name: string;
  builtin: boolean;
  hotkey: string | null;
  autoApps: string[]; // exe name (Windows) or bundle id (macOS)
  bindings: Record<string, Binding>; // key code -> binding
};

export type SwitchMode = "manual" | "hotkey" | "auto";

export type Hotkeys = { nextProfile: string; prevProfile: string; show: string };

export type Settings = {
  schema: number;
  activeProfile: string;
  switchMode: SwitchMode;
  hotkeys: Hotkeys;
  startAtLogin: boolean;
  closeToTray: boolean;
  runAsAdmin: boolean;
  windowsLegends: boolean;
  autoCheckUpdates: boolean;
};

export type AppEntry = {
  id: string;
  name: string;
  kind: "aumid" | "path" | "bundle" | "steam";
  launch: string;
  icon: string | null; // PNG path in the data dir's icons/ folder
  source: "windows" | "macos" | "steam";
};

export type KeyEvent = { code: string; mods: string[]; down: boolean; raw: string };

export type Status = {
  activeProfile: string;
  hookRunning: boolean;
  deviceFound: boolean;
  version: string;
  platform: "windows" | "macos";
  permissionsOk: boolean; // macOS Accessibility + Input Monitoring; always true on Windows
};

/** Payload of "update-available" and the result of check_update. */
export type UpdateInfo = { version: string; notes: string };

export type CatalogProgress = { done: number; total: number; source: string };
