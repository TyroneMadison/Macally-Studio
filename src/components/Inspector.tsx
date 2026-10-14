import { useState } from "react";
import ActionPicker from "./ActionPicker";
import ChordField from "./ChordField";
import KeyMonitor from "./KeyMonitor";
import { describeAction, formatChord, isComplete, isMac } from "../lib/actions";
import type { Action, AppEntry, Binding, KeyEvent, Layout, Modifier, Profile } from "../lib/types";

export type InspectorProps = {
  keyCode: string | null;
  layout: Layout;
  profile: Profile;
  onChange: (p: Profile) => void;
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

/** Key / Combo edits inline; every other pill opens the picker on its tab (empty template or current action). */
const PILLS: { label: string; types: Action["type"][]; template: Action }[] = [
  { label: "Key / Combo", types: ["key_combo"], template: { type: "key_combo", keys: [] } },
  { label: "App", types: ["launch_app"], template: { type: "launch_app", app: { kind: "path", path: "", args: "" } } },
  { label: "Steam game", types: ["steam_game"], template: { type: "steam_game", appid: 0, name: "" } },
  { label: "URL / File", types: ["open"], template: { type: "open", target: "" } },
  { label: "Command", types: ["command"], template: { type: "command", program: "", args: [], hidden: true } },
  { label: "Profile", types: ["switch_profile", "pause_toggle"], template: { type: "switch_profile", profile: "" } },
];

export default function Inspector(props: InspectorProps) {
  const { keyCode, profile, lastKey } = props;
  return (
    <aside className="kb-panel insp" aria-label="Key inspector">
      {keyCode ? (
        // keyed: switching key or profile drops any unsaved edit
        <KeyEditor key={`${profile.id}/${keyCode}`} {...props} keyCode={keyCode} />
      ) : (
        <>
          <p className="muted">Click a key to inspect it.</p>
          <div className="insp-spacer" />
        </>
      )}
      <KeyMonitor last={lastKey} />
    </aside>
  );
}

function KeyEditor({ keyCode, layout, profile, onChange, catalog, profiles }: InspectorProps & { keyCode: string }) {
  const saved: Binding | undefined = profile.bindings[keyCode];
  const [draft, setDraft] = useState<Binding | null>(null); // unsaved edit
  const [picking, setPicking] = useState<Action | null>(null);
  const shown = draft ?? saved;
  const mods = shown?.mods ?? [];
  const key = layout.keys.find((k) => k.code === keyCode);
  const summary = describeAction(shown?.action, catalog, profiles);
  const readOnly = profile.builtin;

  const edit = (action: Action) => setDraft((d) => ({ mods: (d ?? saved)?.mods ?? [], action }));

  function commit(b: Binding | null) {
    const bindings = { ...profile.bindings };
    if (b) bindings[keyCode] = b;
    else delete bindings[keyCode];
    onChange({ ...profile, bindings });
    setDraft(null);
  }

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
        <div className="kb-label">{draft ? "Not saved yet" : "Currently bound to"}</div>
        <div className="insp-bound">
          <span className="insp-type">{summary.type}</span>
          <span className="insp-title">{summary.title}</span>
        </div>
        {summary.detail && <code className="insp-detail">{summary.detail}</code>}
        {mods.length > 0 && <span className="muted">Only while holding {formatChord(mods)}</span>}
      </div>

      <div>
        <div className="kb-label" id="insp-types">Action type</div>
        <div className="insp-pills" role="group" aria-labelledby="insp-types">
          {PILLS.map((p) => {
            const on = shown !== undefined && p.types.includes(shown.action.type);
            const current = on && shown ? shown.action : p.template;
            return (
              <button
                key={p.label}
                type="button"
                aria-pressed={on}
                disabled={readOnly}
                onClick={() => (p.types[0] === "key_combo" ? edit(current) : setPicking(current))}
              >
                {p.label}
              </button>
            );
          })}
        </div>
      </div>

      {shown?.action.type === "key_combo" && (
        <div className="insp-field">
          <label className="kb-label" htmlFor="insp-chord">Send keys</label>
          <ChordField
            id="insp-chord"
            initial={shown.action.keys}
            disabled={readOnly}
            onChange={(keys) => edit({ type: "key_combo", keys })}
          />
          <p className="muted">Click Record, then press the keys you want this key to send.</p>
        </div>
      )}

      <div className="insp-field">
        <label className="kb-label" htmlFor="insp-held">Only when held with</label>
        <select
          id="insp-held"
          className="input"
          value={mods[0] ?? ""}
          disabled={readOnly || !shown}
          onChange={(e) => {
            const m = e.target.value as Modifier | "";
            if (shown) setDraft({ mods: m ? [m] : [], action: shown.action });
          }}
        >
          {HELD.map((h) => (
            <option key={h.value} value={h.value}>
              {h.label}
            </option>
          ))}
        </select>
      </div>

      {readOnly && <p className="muted">Built-in profiles are read-only. Duplicate it in Profiles to edit.</p>}
      <div className="insp-spacer" />

      <div className="insp-actions">
        <button
          type="button"
          className="btn primary"
          disabled={readOnly || !draft || !isComplete(draft.action)}
          onClick={() => draft && commit(draft)}
        >
          Save binding
        </button>
        <button
          type="button"
          className="btn"
          disabled={readOnly || (!saved && !draft)}
          onClick={() => (saved ? commit(null) : setDraft(null))}
        >
          Clear
        </button>
      </div>

      <ActionPicker
        open={picking !== null}
        keyCode={keyCode}
        initial={picking ?? undefined}
        catalog={catalog}
        profiles={profiles}
        onPick={(a) => {
          setPicking(null);
          commit({ mods, action: a }); // "Bind to <key>" saves straight away
        }}
        onClose={() => setPicking(null)}
      />
    </>
  );
}
