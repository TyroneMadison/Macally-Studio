import { useCallback, useState } from "react";
import Keyboard3D from "../components/Keyboard3D";
import type { Layout } from "../lib/types";
import { clampView, DEFAULT_VIEW, flipYaw, type View3D } from "../lib/view";

// Part E swaps this for the full AppState; only the layout is needed until then.
export type KeyboardStore = { layout: Layout };

const NO_BINDINGS = new Set<string>();
const DEG = 180 / Math.PI;

export default function Keyboard({ store }: { store: KeyboardStore }) {
  const [selected, setSelected] = useState<string | null>(null);
  const [legends, setLegends] = useState<"mac" | "win">("mac");
  const [view, setView] = useState<View3D>(DEFAULT_VIEW);
  const onViewChange = useCallback((v: View3D) => setView(clampView(v)), []);
  const key = store.layout.keys.find((k) => k.code === selected);

  return (
    <div className="kb">
      <header className="kb-head">
        <h1>Keyboard</h1>
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
              boundCodes={NO_BINDINGS}
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
        <aside className="kb-panel" aria-label="Key inspector">
          {key ? (
            <>
              <div className="kb-label">Selected key</div>
              <code className="kb-code">{key.code}</code>
              <div className="muted">
                Printed: {key.mac || "space"} · Windows: {key.win || "Space"}
              </div>
              {key.verify && <div className="muted">OS code not yet confirmed on hardware.</div>}
            </>
          ) : (
            <p className="muted">Click a key to inspect it.</p>
          )}
        </aside>
      </section>
    </div>
  );
}
