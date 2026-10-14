// Typed wrappers over Tauri invoke/listen. Command names are the Rust fn names in commands.rs.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppEntry, CatalogProgress, KeyEvent, Layout, Profile, Settings, Status, UpdateInfo } from "./types";

export const getLayout = () => invoke<Layout>("get_layout");
export const getProfiles = () => invoke<Profile[]>("get_profiles");
export const getProfile = (id: string) => invoke<Profile>("get_profile", { id });
export const saveProfile = (profile: Profile) => invoke<void>("save_profile", { profile });
export const deleteProfile = (id: string) => invoke<void>("delete_profile", { id });
export const duplicateProfile = (id: string) => invoke<Profile>("duplicate_profile", { id });
export const importProfile = (path: string) => invoke<Profile>("import_profile", { path });
export const exportProfile = (id: string, path: string) => invoke<void>("export_profile", { id, path });
export const setActiveProfile = (id: string) => invoke<void>("set_active_profile", { id });
export const getSettings = () => invoke<Settings>("get_settings");
export const saveSettings = (settings: Settings) => invoke<void>("save_settings", { settings });
export const setPaused = (paused: boolean) => invoke<void>("set_paused", { paused });
export const getCatalog = () => invoke<AppEntry[]>("get_catalog");
export const refreshCatalog = () => invoke<AppEntry[]>("refresh_catalog");
export const addCatalogEntry = (path: string) => invoke<AppEntry[]>("add_catalog_entry", { path });

// Record and Monitor share the engine's one mode. The Monitor wish lives here, so ending a Record
// (from any field, mounted or not) puts Monitor back exactly when the Keyboard view still wants it.
let monitorWanted = false;
let recording = false;
export const startRecord = () => {
  recording = true;
  return invoke<void>("start_record");
};
export const stopRecord = () => {
  recording = false;
  return invoke<void>("stop_record").then(() =>
    monitorWanted && !recording ? invoke<void>("set_monitor", { on: true }) : undefined,
  );
};
export const setMonitor = (on: boolean) => {
  monitorWanted = on;
  return recording ? Promise.resolve() : invoke<void>("set_monitor", { on });
};
export const checkUpdate = () => invoke<UpdateInfo | null>("check_update");
export const installUpdate = () => invoke<void>("install_update");
export const getStatus = () => invoke<Status>("get_status");

const on =
  <T>(event: string) =>
  (cb: (payload: T) => void): Promise<UnlistenFn> =>
    listen<T>(event, (e) => cb(e.payload));

export const onKeyEvent = on<KeyEvent>("key-event");
export const onProfileChanged = on<{ id: string }>("profile-changed");
export const onPausedChanged = on<{ paused: boolean }>("paused-changed");
export const onCatalogProgress = on<CatalogProgress>("catalog-progress");
export const onUpdateAvailable = on<UpdateInfo>("update-available");
export const onForegroundChanged = on<{ name: string }>("foreground-changed");
