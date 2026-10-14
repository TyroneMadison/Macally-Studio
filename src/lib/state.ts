// App-wide data loaded from Rust, kept fresh by Tauri events. Views get it as `store`.
import { createContext, createElement, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import * as ipc from "./ipc";
import layoutJson from "./layout.json";
import type { AppEntry, KeyEvent, Layout, Profile, Settings, Status, UpdateInfo } from "./types";

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
  refresh(): Promise<void>;
  setActiveProfile(id: string): Promise<void>;
  saveProfile(p: Profile): Promise<void>;
  saveSettings(s: Settings): Promise<void>;
  setPaused(b: boolean): Promise<void>;
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

  const refresh = useCallback(async () => setData(await load()), []);

  useEffect(() => {
    refresh().catch((e) => setError(String(e)));
    const subs = [
      ipc.onKeyEvent(setLastKey),
      ipc.onProfileChanged(() => void refresh()),
      ipc.onPausedChanged(({ paused }) =>
        setData((d) => (d?.status ? { ...d, status: { ...d.status, paused } } : d)),
      ),
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
    return {
      ...data,
      layout,
      activeProfile: data.profiles.find((p) => p.id === data.settings.activeProfile) ?? null,
      lastKey,
      update,
      refresh,
      setActiveProfile: (id) => thenRefresh(ipc.setActiveProfile(id)),
      saveProfile: (p) => thenRefresh(ipc.saveProfile(p)),
      saveSettings: (s) => thenRefresh(ipc.saveSettings(s)),
      setPaused: (b) => thenRefresh(ipc.setPaused(b)),
    };
  }, [data, lastKey, update, refresh]);

  if (error) return createElement("p", { className: "load-error" }, `Could not load Macally Studio data: ${error}`);
  if (!value) return null;
  return createElement(Ctx.Provider, { value }, children);
}
