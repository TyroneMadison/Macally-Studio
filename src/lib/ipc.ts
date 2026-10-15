// Typed wrappers over Tauri invoke/listen. Command names are the Rust fn names in commands.rs.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppEntry, CatalogProgress, KeyEvent, Layout, Profile, Settings, Status, UpdateInfo } from "./types";

export const getLayout = () => invoke<Layout>("get_layout");
export const getProfiles = () => invoke<Profile[]>("get_profiles");
export const getProfile = (id: string) => invoke<Profile>("get_profile", { id });
export const saveProfile = (profile: Profile) => invoke<void>("save_profile", { profile });
export const deleteProfile = (id: string) => invoke<void>("delete_profile", { id });
/** Puts a built-in profile back to its shipped bindings. */
export const resetProfile = (id: string) => invoke<void>("reset_profile", { id });
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

/** Key Monitor: the engine reports every key to the UI while on (and while the window shows). */
export const setMonitor = (on: boolean) => invoke<void>("set_monitor", { on });
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
