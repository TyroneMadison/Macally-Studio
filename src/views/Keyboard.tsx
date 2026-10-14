import { useCallback, useEffect, useMemo, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import Inspector from "../components/Inspector";
import Keyboard3D from "../components/Keyboard3D";
import { setMonitor } from "../lib/ipc";
import { BACKLIGHT_LEVELS, loadLighting, saveLighting } from "../lib/lighting";
import { useRecord } from "../lib/record";
import type { AppState } from "../lib/state";
import { clampView, DEFAULT_VIEW, flipYaw, PITCH_MAX, PITCH_MIN, ZOOM_MAX, type View3D } from "../lib/view";

const DEG = 180 / Math.PI;

type SliderProps = { label: string; min: number; max: number; unit: string; value: number; onChange: (v: number) => void };

function Slider({ label, min, max, unit, value, onChange }: SliderProps) {
  return (
    <label>
      {label}
      <input
        type="range"
        min={min}
        max={max}
        step={1}
        aria-label={label}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
      />
      <span className="mono">
        {value}
        {unit}
      </span>
    </label>
  );
}

export default function Keyboard({ store }: { store: AppState }) {
  const [selected, setSelected] = useState<string | null>(null);
  const [picking, setPicking] = useState(false);
  const [view, setView] = useState<View3D>(DEFAULT_VIEW);
  const onViewChange = useCallback((v: View3D) => setView(clampView(v)), []);
  const setPart = (p: Partial<View3D>) => setView((v) => clampView({ ...v, ...p }));
  const [lighting, setLighting] = useState(loadLighting);
  useEffect(() => saveLighting(lighting), [lighting]);
  const profile = store.activeProfile;
  const boundCodes = useMemo(() => new Set(Object.keys(profile?.bindings ?? {})), [profile]);
  const legends: "mac" | "win" = store.settings.windowsLegends ? "win" : "mac";
  const setLegends = (l: "mac" | "win") => void store.saveSettings({ windowsLegends: l === "win" });

  // Monitor mode while this view is mounted, so the Key Monitor strip sees every press. Rust keeps it
  // off while the window is hidden and ends it on close-to-tray; focus coming back turns it on again.
  useEffect(() => {
    let live = true;
    const on = () => void setMonitor(true).catch(console.error);
    on();
    const off = getCurrentWindow().onFocusChanged(({ payload: focused }) => live && focused && on());
    return () => {
      live = false;
      void off.then((unlisten) => unlisten()).catch(() => undefined);
      setMonitor(false).catch(console.error);
    };
  }, []);

  // Pick key by pressing: Record mode, so the press selects the key instead of running its binding
  useRecord(
    picking,
    () => (e) => {
      const hit = e.down && store.layout.keys.some((k) => k.code === e.code);
      if (hit) setSelected(e.code);
      return hit;
    },
    () => setPicking(false),
  );

  return (
    <div className="kb">
      <header className="kb-head">
        <h1>Keyboard</h1>
        <span className="muted">{profile ? profile.name : "No active profile"}</span>
        {/* Fn-Lock: without it the top row sends the keyboard's own macros, not F1-F15 */}
        <span className="muted small">Tip: press fn + esc so the top row sends F1-F15</span>
        <div className="kb-spacer" />
        <button
          type="button"
          className={"btn" + (picking ? " rec" : "")}
          aria-pressed={picking}
          onClick={() => setPicking((p) => !p)}
        >
          {picking ? "Press any key…" : "Pick key by pressing"}
        </button>
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
              pitch={view.pitch}
              zoom={view.zoom}
              lighting={lighting}
              onViewChange={onViewChange}
            />
            <div className="kb-hint">Right-drag to rotate · Scroll to zoom</div>
          </div>
          <div className="kb-bar">
            <Slider
              label="Rotate"
              min={-180}
              max={180}
              unit="°"
              value={Math.round(view.yaw * DEG)}
              onChange={(d) => setPart({ yaw: d / DEG })}
            />
            {/* tilt reads as the camera's height above the desk: 85° is almost straight down */}
            <Slider
              label="Tilt"
              min={Math.round(90 - PITCH_MAX * DEG)}
              max={Math.round(90 - PITCH_MIN * DEG)}
              unit="°"
              value={Math.round(90 - view.pitch * DEG)}
              onChange={(d) => setPart({ pitch: (90 - d) / DEG })}
            />
            <Slider
              label="Zoom"
              min={100}
              max={ZOOM_MAX * 100}
              unit="%"
              value={Math.round(view.zoom * 100)}
              onChange={(p) => setPart({ zoom: p / 100 })}
            />
            <div className="kb-actions">
              <button type="button" className="btn" onClick={() => setPart({ yaw: flipYaw(view.yaw) })}>
                Flip 180°
              </button>
              <button type="button" className="btn" onClick={() => setView(DEFAULT_VIEW)}>
                Reset view
              </button>
              <details className="kb-pop">
                <summary className="btn">Lighting</summary>
                <div className="kb-pop-body">
                  <Slider
                    label="Brightness"
                    min={50}
                    max={200}
                    unit="%"
                    value={Math.round(lighting.brightness * 100)}
                    onChange={(p) => setLighting((l) => ({ ...l, brightness: p / 100 }))}
                  />
                  <div className="kb-pop-row">
                    Backlight
                    <div className="seg" role="group" aria-label="Backlight">
                      {BACKLIGHT_LEVELS.map((name, i) => (
                        <button
                          key={name}
                          type="button"
                          aria-pressed={lighting.backlight === i}
                          onClick={() => setLighting((l) => ({ ...l, backlight: i }))}
                        >
                          {name}
                        </button>
                      ))}
                    </div>
                  </div>
                </div>
              </details>
            </div>
          </div>
        </div>
        {profile ? (
          <Inspector
            keyCode={selected}
            layout={store.layout}
            profile={profile}
            saveBinding={store.saveBinding}
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
