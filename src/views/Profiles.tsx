import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import type { View } from "../components/Rail";
import { importProfile } from "../lib/ipc";
import { initials, newProfile } from "../lib/profile";
import type { AppState } from "../lib/state";
import type { Profile, SwitchMode } from "../lib/types";
import ProfileDetail from "./ProfileDetail";

const MODE_LABEL: Record<SwitchMode, string> = { manual: "Manual", hotkey: "Hotkey", auto: "Auto per-app" };

// 24x24 stroke icons for the two built-ins; every other profile shows its initials
const BUILTIN_ICON: Record<string, string> = {
  "windows-default": "M3 3h8v8H3zM13 3h8v8h-8zM3 13h8v8H3zM13 13h8v8h-8z",
  "macos-default": "M12 3c-4 4-4 10 0 14 4-4 4-10 0-14zM5 20c4-2 10-2 14 0",
};

function Icon({ d }: { d: string }) {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d={d} />
    </svg>
  );
}

type Props = { store: AppState; onNav?: (v: View) => void };

export default function Profiles({ store, onNav }: Props) {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const activeId = store.activeProfile?.id;
  const selected = store.profiles.find((p) => p.id === selectedId) ?? store.activeProfile ?? store.profiles[0];

  const run = (task: Promise<unknown>) => {
    setError(null);
    task.catch((e) => setError(String(e)));
  };
  const importFile = async () => {
    const path = await open({ multiple: false, filters: [{ name: "Macally Studio profile", extensions: ["json"] }] });
    if (!path) return;
    const p = await importProfile(path);
    await store.refresh();
    setSelectedId(p.id);
  };
  const create = async () => {
    const p = newProfile(store.profiles);
    await store.saveProfile(p);
    setSelectedId(p.id);
  };
  // The keyboard view edits the active profile, so make this one active first.
  const editKeys = async (p: Profile) => {
    if (p.id !== activeId) await store.setActiveProfile(p.id);
    onNav?.("keyboard");
  };

  const card = (p: Profile) => (
    <button
      key={p.id}
      type="button"
      className={"profile-card" + (p.id === selected?.id ? " selected" : "")}
      aria-pressed={p.id === selected?.id}
      onClick={() => setSelectedId(p.id)}
    >
      <span className={"glyph" + (p.id === activeId ? " lit" : "")}>
        {BUILTIN_ICON[p.id] ? <Icon d={BUILTIN_ICON[p.id]} /> : initials(p.name)}
      </span>
      <span className="col grow gap-6">
        <span className="profile-name">
          {p.name}
          {p.id === activeId && <span className="pill">Active</span>}
        </span>
        <span className="row wrap gap-6">
          <span className="muted small">{Object.keys(p.bindings).length} keys bound</span>
          {p.autoApps.map((a) => (
            <span key={a} className="chip">
              {a}
            </span>
          ))}
        </span>
      </span>
      <span className="muted small mono">{p.hotkey}</span>
    </button>
  );

  const mine = store.profiles.filter((p) => !p.builtin);

  return (
    <div className="page">
      <header className="page-head">
        <h1>Profiles</h1>
        <span className="muted">Switching:</span>
        <span className="chip">{MODE_LABEL[store.settings.switchMode]}</span>
        <button type="button" className="link" onClick={() => onNav?.("settings")}>
          Change
        </button>
        <div className="kb-spacer" />
        {error && <span className="error small">{error}</span>}
        <button type="button" className="btn" onClick={() => run(importFile())}>
          <Icon d="M12 3v12M6 9l6-6 6 6M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2" />
          Import
        </button>
        <button type="button" className="btn primary" onClick={() => run(create())}>
          <Icon d="M12 5v14M5 12h14" />
          New profile
        </button>
      </header>
      <section className="page-body">
        <div className="profile-list">
          <div className="kb-label">Built-in</div>
          {store.profiles.filter((p) => p.builtin).map(card)}
          <div className="kb-label">Mine</div>
          {mine.length > 0 ? mine.map(card) : <p className="muted small">No profiles yet. Duplicate a built-in or create a new one.</p>}
        </div>
        {selected && (
          <ProfileDetail
            key={selected.id}
            store={store}
            profile={selected}
            onSelect={setSelectedId}
            onEditKeys={() => run(editKeys(selected))}
          />
        )}
      </section>
    </div>
  );
}
