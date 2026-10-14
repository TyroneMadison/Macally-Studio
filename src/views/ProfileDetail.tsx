import { useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import AppRulePicker from "../components/AppRulePicker";
import HotkeyField from "../components/HotkeyField";
import { deleteProfile, duplicateProfile, exportProfile } from "../lib/ipc";
import { actionLabel, initials, ruleFor } from "../lib/profile";
import type { AppState } from "../lib/state";
import type { Profile } from "../lib/types";

type Props = {
  store: AppState;
  profile: Profile;
  onSelect: (id: string) => void;
  onEditKeys: () => void;
};

/** Right-hand panel of the Profiles view. Mount with key={profile.id} so local edits reset per profile. */
export default function ProfileDetail({ store, profile: p, onSelect, onEditKeys }: Props) {
  const [name, setName] = useState(p.name);
  const [picking, setPicking] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const readOnly = p.builtin;
  const active = store.activeProfile?.id === p.id;
  const legend = store.settings.windowsLegends ? "win" : "mac";

  const run = (task: Promise<unknown>) => {
    setError(null);
    task.catch((e) => setError(String(e)));
  };
  const update = (patch: Partial<Profile>) => run(store.saveProfile({ ...patch, id: p.id }));
  const appName = (rule: string) => store.catalog.find((a) => ruleFor(a) === rule)?.name ?? rule;
  const keyName = (code: string) => store.layout.keys.find((k) => k.code === code)?.[legend] || code;

  const rename = () => {
    const next = name.trim();
    if (next && next !== p.name) update({ name: next });
    else setName(p.name);
  };
  const addRule = (rule: string) => {
    setPicking(false);
    if (!p.autoApps.includes(rule)) update({ autoApps: [...p.autoApps, rule] });
  };
  const duplicate = async () => {
    const copy = await duplicateProfile(p.id);
    await store.refresh();
    onSelect(copy.id);
  };
  const exportFile = async () => {
    const path = await save({
      defaultPath: `${p.id}.json`,
      filters: [{ name: "Macally Studio profile", extensions: ["json"] }],
    });
    if (path) await exportProfile(p.id, path);
  };
  const remove = async () => {
    await deleteProfile(p.id);
    await store.refresh();
  };

  return (
    <aside className="detail" aria-label="Profile details">
      <div className="row gap-12">
        <span className="glyph">{initials(p.name)}</span>
        <div className="field grow">
          <label htmlFor="profile-name" className="kb-label">
            Name
          </label>
          <input
            id="profile-name"
            className="input strong"
            value={name}
            disabled={readOnly}
            onChange={(e) => setName(e.target.value)}
            onBlur={rename}
            onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()}
          />
        </div>
      </div>

      <div className="field">
        <div className="kb-label">Activate automatically when</div>
        {p.autoApps.map((rule) => (
          <div key={rule} className="rule">
            <span className="glyph xs">{initials(appName(rule))}</span>
            <span className="grow strong">{appName(rule)} is in front</span>
            {!readOnly && (
              <button
                type="button"
                className="icon-btn"
                aria-label={`Remove ${appName(rule)}`}
                onClick={() => update({ autoApps: p.autoApps.filter((r) => r !== rule) })}
              >
                ×
              </button>
            )}
          </div>
        ))}
        {readOnly ? (
          p.autoApps.length === 0 && <div className="muted small">Duplicate this profile to add app rules.</div>
        ) : (
          <button type="button" className="btn dashed" onClick={() => setPicking(true)}>
            + Add app rule
          </button>
        )}
      </div>

      <div className="field">
        <label htmlFor="profile-hotkey" className="kb-label">
          Hotkey to activate
        </label>
        <HotkeyField
          id="profile-hotkey"
          value={p.hotkey ?? ""}
          disabled={readOnly}
          onChange={(v) => update({ hotkey: v || null })}
        />
      </div>

      <div className="field">
        <div className="kb-label">Bindings in this profile</div>
        {Object.keys(p.bindings).length === 0 ? (
          <div className="muted small">No keys remapped. Every key does what its legend says.</div>
        ) : (
          <div className="binding-grid">
            {Object.entries(p.bindings).map(([code, b]) => (
              <div key={code} className="binding">
                <span className="mono strong">{[...b.mods, keyName(code)].join("+")}</span>
                <span className="muted ellipsis">{actionLabel(b.action, store.catalog, store.profiles)}</span>
              </div>
            ))}
          </div>
        )}
        <button type="button" className="link" onClick={onEditKeys}>
          Edit on keyboard
        </button>
      </div>

      <div className="grow" />
      {error && <div className="error small">{error}</div>}
      <div className="row gap-8 wrap">
        <button
          type="button"
          className="btn primary grow"
          disabled={active}
          onClick={() => run(store.setActiveProfile(p.id))}
        >
          {active ? "Active" : "Activate"}
        </button>
        <button type="button" className="btn" onClick={() => run(duplicate())}>
          Duplicate
        </button>
        <button type="button" className="btn" onClick={() => run(exportFile())}>
          Export
        </button>
        {!readOnly && (
          <button
            type="button"
            className="btn danger"
            onClick={() => (confirmDelete ? run(remove()) : setConfirmDelete(true))}
          >
            {confirmDelete ? "Confirm delete" : "Delete"}
          </button>
        )}
      </div>

      {picking && <AppRulePicker catalog={store.catalog} onPick={addRule} onClose={() => setPicking(false)} />}
    </aside>
  );
}
