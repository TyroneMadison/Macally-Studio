import { useEffect, useState } from "react";
import ActionPicker from "./ActionPicker";
import KeyMonitor from "./KeyMonitor";
import Keycap from "./Keycap";
import KeyPicker from "./KeyPicker";
import FnSection from "./FnSection";
import { describeAction, formatChord, isMac } from "../lib/actions";
import { isFirmwareOnly } from "../lib/keys";
import type { AppState } from "../lib/state";
import type { AppEntry, Binding, KeyEvent, Layout, Modifier, Profile } from "../lib/types";

export type InspectorProps = {
  keyCode: string | null;
  layout: Layout;
  profile: Profile;
  saveBinding: AppState["saveBinding"];
  catalog: AppEntry[];
  profiles: Profile[];
  lastKey: KeyEvent | null;
  /** What the key does when this profile leaves it alone (a game's profile runs on the Default). */
  fallback?: (code: string) => Binding | undefined;
  resetKeyEverywhere: AppState["resetKeyEverywhere"];
  /** Selects another key (the Fn section points at the keys next to fn). */
  onSelectKey?: (code: string) => void;
};

/** Values are `Modifier` (the Rust variant names, unrenamed); one modifier per binding from this UI. */
const HELD: { value: Modifier | ""; label: string }[] = [
  { value: "", label: "Nothing (plain press)" },
  { value: "Shift", label: "Shift" },
  { value: "Ctrl", label: isMac ? "Control" : "Ctrl" },
  { value: "Alt", label: isMac ? "Option" : "Alt" },
  { value: "Meta", label: isMac ? "Command" : "Win" },
];

export default function Inspector(props: InspectorProps) {
  const { keyCode, profile, lastKey } = props;
  return (
    <aside className="kb-panel insp" aria-label="Key inspector">
      {keyCode ? (
        isFirmwareOnly(keyCode) ? (
          <FnEditor key={`${profile.id}/${keyCode}`} {...props} keyCode={keyCode} />
        ) : (
          // keyed: switching key or profile drops the "Saved" note
          <KeyEditor key={`${profile.id}/${keyCode}`} {...props} keyCode={keyCode} />
        )
      ) : (
        <>
          <p className="muted">Click a key on the keyboard to change it.</p>
          <div className="insp-spacer" />
        </>
      )}
      <KeyMonitor last={lastKey} />
    </aside>
  );
}

/** The inspector for a firmware-only key (Fn): its header, then the honest Fn explainer. */
function FnEditor({ keyCode, layout, profile, saveBinding, resetKeyEverywhere, onSelectKey }: InspectorProps & { keyCode: string }) {
  const key = layout.keys.find((k) => k.code === keyCode);
  return (
    <>
      <div className="insp-key">
        <Keycap code={keyCode} size="lg" lit />
        <div>
          <div className="insp-name">{keyCode}</div>
          <div className="muted">
            Printed: {key?.mac || "space"} · Windows: {key?.win || "Space"}
          </div>
        </div>
      </div>
      <FnSection profile={profile} saveBinding={saveBinding} resetKeyEverywhere={resetKeyEverywhere} onSelectKey={onSelectKey} />
      <div className="insp-spacer" />
    </>
  );
}

/** Every change saves at once through `saveBinding`. */
function KeyEditor({ keyCode, layout, profile, saveBinding, catalog, profiles, fallback }: InspectorProps & { keyCode: string }) {
  const binding: Binding | undefined = profile.bindings[keyCode];
  const inherited = binding ? undefined : fallback?.(keyCode);
  const mods = binding?.mods ?? [];
  const off = binding?.action.type === "block";
  const [picking, setPicking] = useState(false);
  const [savedAt, setSavedAt] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const key = layout.keys.find((k) => k.code === keyCode);
  const summary = describeAction((binding ?? inherited)?.action, catalog, profiles);
  const sends = binding?.action.type === "key_combo" ? binding.action.keys : [];

  /** Built from the latest saved binding, not this render's, so quick edits cannot undo each other. */
  function commit(update: (old?: Binding) => Binding | null) {
    setError(null);
    saveBinding(profile.id, keyCode, update).then(
      () => setSavedAt(Date.now()),
      (e) => setError(String(e)),
    );
  }
  const send = (keys: string[]) => commit((old) => ({ mods: old?.mods ?? [], action: { type: "key_combo", keys } }));

  useEffect(() => {
    if (!savedAt) return;
    const t = setTimeout(() => setSavedAt(0), 1500);
    return () => clearTimeout(t);
  }, [savedAt]);

  return (
    <>
      <div className="insp-key">
        <Keycap code={keyCode} size="lg" lit />
        <div>
          <div className="insp-name">{keyCode}</div>
          <div className="muted">
            Printed: {key?.mac || "space"} · Windows: {key?.win || "Space"}
          </div>
        </div>
      </div>
      {key?.mac === "fn" ? (
        <p className="muted">
          Printed fn: on Mac keyboards this is often a second fn key that the keyboard handles itself. Press it and watch
          the Key Monitor below; if nothing shows up, Windows never sees it and a remap here can’t work.
        </p>
      ) : (
        key?.verify && <p className="muted">OS code not yet confirmed on hardware.</p>
      )}

      <div className="insp-card">
        <div className="kb-label">{inherited ? "From your Default profile" : "Currently bound to"}</div>
        <div className="insp-bound">
          <span className="insp-type">{summary.type}</span>
          <span className="insp-title">{summary.title}</span>
        </div>
        {summary.detail && <code className="insp-detail">{summary.detail}</code>}
        {mods.length > 0 && <span className="muted">Only while holding {formatChord(mods)}</span>}
      </div>

      <div className="insp-field">
        <div className="kb-label">Remap to</div>
        <KeyPicker
          value={sends}
          onPick={send}
          action={binding?.action}
          catalog={catalog}
          onLaunch={(a) => commit((old) => ({ mods: old?.mods ?? [], action: a }))}
        />
        <p className="muted">Choose what it sends: a key, a Windows shortcut like Win + V, or a program to launch.</p>
      </div>

      <div className="insp-field">
        <label className="kb-label" htmlFor="insp-held">Only when held with</label>
        <select
          id="insp-held"
          className="input"
          value={mods[0] ?? ""}
          disabled={!binding}
          onChange={(e) => {
            const m = e.target.value as Modifier | "";
            commit((old) => (old ? { mods: m ? [m] : [], action: old.action } : null));
          }}
        >
          {HELD.map((h) => (
            <option key={h.value} value={h.value}>
              {h.label}
            </option>
          ))}
        </select>
      </div>

      <div className="insp-actions">
        <button
          type="button"
          className="btn"
          onClick={() => setPicking(true)}
        >
          More actions…
        </button>
        <button
          type="button"
          className="btn"
          onClick={() => commit((old) => (off ? null : { mods: old?.mods ?? [], action: { type: "block" } }))}
        >
          {off ? "Turn key on" : "Turn key off"}
        </button>
        <button type="button" className="btn" disabled={!binding} onClick={() => commit(() => null)}>
          Reset key
        </button>
      </div>
      <p className={error ? "error small" : "muted small"} role="status">
        {error ?? (savedAt ? "Saved" : "")}
      </p>
      <div className="insp-spacer" />

      <ActionPicker
        open={picking}
        keyCode={keyCode}
        initial={binding?.action}
        catalog={catalog}
        profiles={profiles}
        onPick={(a) => {
          setPicking(false);
          commit((old) => ({ mods: old?.mods ?? [], action: a }));
        }}
        onClose={() => setPicking(false)}
      />
    </>
  );
}
