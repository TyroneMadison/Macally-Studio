// App-wide data loaded from Rust, kept fresh by Tauri events. Views get it as `store`.
import { createContext, createElement, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import * as ipc from "./ipc";
import layoutJson from "./layout.json";
import { withBinding } from "./profile";
import type { AppEntry, Binding, KeyEvent, Layout, Profile, Settings, Status, Switched, UpdateInfo } from "./types";

const layout: Layout = layoutJson;

export type AppState = {
  profiles: Profile[];
  settings: Settings;
  activeProfile: Profile | null;
  catalog: AppEntry[];
  status: Status | null; // null until get_status is available
  layout: Layout;
  lastKey: KeyEvent | null;
  update: UpdateInfo | null; // last "update-available" event
  /** The last automatic switch ("profile-switched") and when it came. */
  switched: (Switched & { at: number }) | null;
  /** The game profile whose game was detected in front; null while none is. */
  liveGame: string | null;
  refresh(): Promise<void>;
  setActiveProfile(id: string): Promise<void>;
  /** Merges `patch` onto the saved profile with that id (a new id saves `patch` as is). */
  saveProfile(patch: Pick<Profile, "id"> & Partial<Profile>): Promise<void>;
  /** Rebinds one key of a saved profile from its latest saved binding (null from `update` unbinds it). */
  saveBinding(id: string, code: string, update: (old?: Binding) => Binding | null): Promise<void>;
  /** Merges `patch` onto the saved settings. */
  saveSettings(patch: Partial<Settings>): Promise<void>;
  /** The game an entry starts gets its own profile (`base`: a profile to copy keys from). */
  addGame(entry: AppEntry, base: string | null): Promise<Profile>;
  removeGame(id: string): Promise<void>;
  /** One game, or every game when `id` is null. */
  setGameAutoSwitch(id: string | null, on: boolean): Promise<void>;
  /** Clears `code` from every profile (Reset Fn key); resolves to how many profiles changed. */
  resetKeyEverywhere(code: string): Promise<number>;
};

type Data = Pick<AppState, "profiles" | "settings" | "catalog" | "status">;

async function load(): Promise<Data> {
  const [profiles, settings, catalog, status] = await Promise.all([
    ipc.getProfiles(),
    ipc.getSettings(),
    // catalog and status commands land with later work; the app must still open without them
    ipc.getCatalog().catch((): AppEntry[] => []),
    ipc.getStatus().catch(() => null),
  ]);
  return { profiles, settings, catalog, status };
}

/** Runs the tasks handed to it one at a time; a failed task does not stop the ones after it. */
export function serial() {
  let tail: Promise<unknown> = Promise.resolve();
  return <T>(task: () => Promise<T>): Promise<T> => {
    const run = tail.then(task);
    tail = run.catch(() => {});
    return run;
  };
}

const Ctx = createContext<AppState | null>(null);

export function useAppState(): AppState {
  const s = useContext(Ctx);
  if (!s) throw new Error("useAppState must be used inside <AppStateProvider>");
  return s;
}

export function AppStateProvider({ children }: { children: ReactNode }) {
  const [data, setData] = useState<Data | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [lastKey, setLastKey] = useState<KeyEvent | null>(null);
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [switched, setSwitched] = useState<AppState["switched"]>(null);
  const [liveGame, setLiveGame] = useState<string | null>(null);

  const latest = useRef<Data | null>(null);
  const [queue] = useState(serial);

  const refresh = useCallback(async () => {
    const d = await load();
    latest.current = d;
    setData(d);
  }, []);

  useEffect(() => {
    refresh().then(
      () => {
        // checked from here, not from Rust at startup, so the result cannot arrive before the UI listens
        if (latest.current?.settings.autoCheckUpdates)
          ipc.checkUpdate().then((u) => u && setUpdate(u)).catch((e) => console.warn("update check failed", e));
      },
      (e) => setError(String(e)),
    );
    const subs = [
      ipc.onKeyEvent(setLastKey),
      ipc.onProfileChanged(({ id }) => {
        // a switch to anything but the detected game's profile ends "live"; a detection arrives right after
        setLiveGame((live) => (live === id ? live : null));
        void refresh();
      }),
      ipc.onProfileSwitched((s) => {
        setSwitched({ ...s, at: Date.now() });
        setLiveGame(s.game ? s.id : null);
      }),
      ipc.onCatalogProgress(({ done, total }) => {
        if (done === total) void refresh();
      }),
      ipc.onUpdateAvailable(setUpdate),
    ];
    return () => subs.forEach((p) => void p.then((unlisten) => unlisten()));
  }, [refresh]);

  const value = useMemo((): AppState | null => {
    if (!data) return null;
    const thenRefresh = (p: Promise<unknown>) => p.then(refresh);
    // Each save merges onto what the previous save reloaded, so a quick second edit
    // (blur a field, then click a switch) cannot write back the first edit's old values.
    const queued = (task: (d: Data) => Promise<unknown>) => queue(() => thenRefresh(task(latest.current!)));
    return {
      ...data,
      layout,
      activeProfile: data.profiles.find((p) => p.id === data.settings.activeProfile) ?? null,
      lastKey,
      update,
      switched,
      liveGame,
      refresh,
      setActiveProfile: (id) => thenRefresh(ipc.setActiveProfile(id)),
      saveProfile: (patch) =>
        queued((d) => ipc.saveProfile({ ...d.profiles.find((p) => p.id === patch.id), ...patch } as Profile)),
      saveBinding: (id, code, update) =>
        queued((d) => ipc.saveProfile(withBinding(d.profiles.find((p) => p.id === id)!, code, update))),
      saveSettings: (patch) => queued((d) => ipc.saveSettings({ ...d.settings, ...patch })),
      addGame: (entry, base) => queue(() => ipc.addGame(entry, base).then(async (p) => (await refresh(), p))),
      removeGame: (id) => queued(() => ipc.removeGame(id)),
      setGameAutoSwitch: (id, on) => queued(() => ipc.setGameAutoSwitch(id, on)),
      resetKeyEverywhere: (code) => queue(() => ipc.resetKeyEverywhere(code).then(async (n) => (await refresh(), n))),
    };
  }, [data, lastKey, update, switched, liveGame, refresh, queue]);

  if (error) return createElement("p", { className: "load-error" }, `Could not load Macally Studio data: ${error}`);
  if (!value) return null;
  return createElement(Ctx.Provider, { value }, children);
}
