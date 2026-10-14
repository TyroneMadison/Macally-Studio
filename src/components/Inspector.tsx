import { useEffect, useState } from "react";
import ActionPicker from "./ActionPicker";
import KeyMonitor from "./KeyMonitor";
import { chordRecorder, describeAction, formatChord, isMac, KEY_GROUPS, keyLabel } from "../lib/actions";
import { useRecord } from "../lib/record";
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
        // keyed: switching key or profile ends a recording and drops the "Saved" note
        <KeyEditor key={`${profile.id}/${keyCode}`} {...props} keyCode={keyCode} />
      ) : (
        <>
          <p className="muted">Click a key on the keyboard, or press 'Pick key by pressing' and press it.</p>
          <div className="insp-spacer" />
        </>
      )}
      <KeyMonitor last={lastKey} />
    </aside>
  );
}

/** Every change saves at once through `saveBinding`. */
function KeyEditor({ keyCode, layout, profile, saveBinding, catalog, profiles }: InspectorProps & { keyCode: string }) {
  const binding: Binding | undefined = profile.bindings[keyCode];
  const mods = binding?.mods ?? [];
  const [recording, setRecording] = useState(false);
  const [picking, setPicking] = useState(false);
  const [savedAt, setSavedAt] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const key = layout.keys.find((k) => k.code === keyCode);
  const summary = describeAction(binding?.action, catalog, profiles);
  const sends = binding?.action.type === "key_combo" ? binding.action.keys : [];
  const listed = sends.length === 1 && KEY_GROUPS.some((g) => g.codes.includes(sends[0])) ? sends[0] : "";

  /** Built from the latest saved binding, not this render's, so quick edits cannot undo each other. */
  function commit(update: (old?: Binding) => Binding | null) {
    setRecording(false); // any other edit ends a recording, so it can never save over that edit
    setError(null);
    saveBinding(profile.id, keyCode, update).then(
      () => setSavedAt(Date.now()),
      (e) => setError(String(e)),
    );
  }
  const send = (keys: string[]) => commit((old) => ({ mods: old?.mods ?? [], action: { type: "key_combo", keys } }));

  useRecord(
    recording,
    () => chordRecorder((keys) => keys.join() !== "Escape" && send(keys)), // a bare Esc cancels
    () => setRecording(false),
  );
  useEffect(() => {
    if (!savedAt) return;
    const t = setTimeout(() => setSavedAt(0), 1500);
    return () => clearTimeout(t);
  }, [savedAt]);

  return (
    <>
      <div className="insp-key">
        <span className="insp-cap">{key?.mac || "space"}</span>
        <div>
          <div className="insp-name">{keyCode}</div>
          <div className="muted">
            Printed: {key?.mac || "space"} · Windows: {key?.win || "Space"}
          </div>
        </div>
      </div>
      {key?.verify && <p className="muted">OS code not yet confirmed on hardware.</p>}

      <div className="insp-card">
        <div className="kb-label">Currently bound to</div>
        <div className="insp-bound">
          <span className="insp-type">{summary.type}</span>
          <span className="insp-title">{summary.title}</span>
        </div>
        {summary.detail && <code className="insp-detail">{summary.detail}</code>}
        {mods.length > 0 && <span className="muted">Only while holding {formatChord(mods)}</span>}
      </div>

      <div className="insp-field">
        <div className="kb-label">Remap to</div>
        <div className="chord">
          <button
            type="button"
            className={recording ? "btn rec" : "btn primary"}
            aria-pressed={recording}
            onClick={() => setRecording((r) => !r)}
          >
            {recording ? "Press any key…" : "Press a key"}
          </button>
          <select
            className="input"
            aria-label="Key to send"
            value={listed}
            onChange={(e) => send([e.target.value])}
          >
            <option value="" disabled>
              Choose a key…
            </option>
            {KEY_GROUPS.map((g) => (
              <optgroup key={g.label} label={g.label}>
                {g.codes.map((c) => (
                  <option key={c} value={c}>
                    {keyLabel(c)}
                  </option>
                ))}
              </optgroup>
            ))}
          </select>
        </div>
        <p className="muted">
          {recording
            ? "Press the key or shortcut it should send. Esc cancels."
            : "Press the key or shortcut it should send, or choose one from the list."}
        </p>
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
          onClick={() => {
            setRecording(false);
            setPicking(true);
          }}
        >
          More actions…
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
