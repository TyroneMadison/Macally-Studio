import { useCallback, useEffect, useMemo, useState } from "react";
import Inspector from "../components/Inspector";
import Keyboard3D from "../components/Keyboard3D";
import { setMonitor } from "../lib/ipc";
import type { AppState } from "../lib/state";
import { clampView, DEFAULT_VIEW, flipYaw, type View3D } from "../lib/view";

const DEG = 180 / Math.PI;

export default function Keyboard({ store }: { store: AppState }) {
  const [selected, setSelected] = useState<string | null>(null);
  const [view, setView] = useState<View3D>(DEFAULT_VIEW);
  const onViewChange = useCallback((v: View3D) => setView(clampView(v)), []);
  const profile = store.activeProfile;
  const boundCodes = useMemo(() => new Set(Object.keys(profile?.bindings ?? {})), [profile]);
  const legends: "mac" | "win" = store.settings.windowsLegends ? "win" : "mac";
  const setLegends = (l: "mac" | "win") => void store.saveSettings({ windowsLegends: l === "win" });

  // Monitor mode while this view is mounted, so the Key Monitor strip sees every press.
  useEffect(() => {
    setMonitor(true).catch(console.error);
    return () => {
      setMonitor(false).catch(console.error);
    };
  }, []);

  return (
    <div className="kb">
      <header className="kb-head">
        <h1>Keyboard</h1>
        <span className="muted">{profile ? profile.name : "No active profile"}</span>
        {/* Fn-Lock: without it the top row sends the keyboard's own macros, not F1-F15 */}
        <span className="muted small">Tip: press fn + esc so the top row sends F1-F15</span>
        <div className="kb-spacer" />
        <div className="seg" role="group" aria-label="Legends">
          <button type="button" aria-pressed={legends === "mac"} onClick={() => setLegends("mac")}>
            Mac legends
          </button>
          <button type="button" aria-pressed={legends === "win"} onClick={() => setLegends("win")}>
            Windows legends
          </button>
        </div>
      </header>
      <section className="kb-body">
        <div className="kb-stage">
          <div className="kb-canvas">
            <Keyboard3D
              layout={store.layout}
              boundCodes={boundCodes}
              selected={selected}
              onSelect={setSelected}
              legends={legends}
              yaw={view.yaw}
              zoom={view.zoom}
              onViewChange={onViewChange}
            />
            <div className="kb-hint">Right-drag to rotate · Scroll to zoom</div>
          </div>
          <div className="kb-bar">
            <label>
              Rotate
              <input
                type="range"
                min={-90}
                max={90}
                step={1}
                aria-label="Rotate keyboard"
                value={Math.round(view.yaw * DEG)}
                onChange={(e) => setView((v) => clampView({ ...v, yaw: Number(e.target.value) / DEG }))}
              />
              <span className="mono">{Math.round(view.yaw * DEG)}°</span>
            </label>
            <label>
              Zoom
              <input
                type="range"
                min={100}
                max={150}
                step={1}
                aria-label="Zoom keyboard"
                value={Math.round(view.zoom * 100)}
                onChange={(e) => setView((v) => clampView({ ...v, zoom: Number(e.target.value) / 100 }))}
              />
              <span className="mono">{Math.round(view.zoom * 100)}%</span>
            </label>
            <div className="kb-spacer" />
            <button type="button" className="btn" onClick={() => setView((v) => ({ ...v, yaw: flipYaw(v.yaw) }))}>
              Flip 180°
            </button>
            <button type="button" className="btn" onClick={() => setView(DEFAULT_VIEW)}>
              Reset view
            </button>
          </div>
        </div>
        {profile ? (
          <Inspector
            keyCode={selected}
            layout={store.layout}
            profile={profile}
            onChange={(p) => void store.saveProfile(p)}
            catalog={store.catalog}
            profiles={store.profiles}
            lastKey={store.lastKey}
          />
        ) : (
          <aside className="kb-panel" aria-label="Key inspector">
            <p className="muted">Loading profiles…</p>
          </aside>
        )}
      </section>
    </div>
  );
}
