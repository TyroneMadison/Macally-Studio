import { useState } from "react";
import AddGame from "../components/AddGame";
import GameTile from "../components/GameTile";
import Keycap from "../components/Keycap";
import { formatChord } from "../lib/actions";
import { bindingCount, chipOf, copyFromDefault, DEFAULT_ID, defaultProfile, gameExe, gamePlace, games } from "../lib/games";
import { duplicateProfile } from "../lib/ipc";
import { actionLabel } from "../lib/profile";
import type { AppState } from "../lib/state";
import type { Profile } from "../lib/types";

type Props = {
  store: AppState;
  /** Opens the full-screen editor for a profile, at one of its keys. */
  onEdit: (profileId: string, key?: string) => void;
};

/** "Game profiles": every game with its own keys, the Default they run on, and adding games. */
export default function Games({ store, onEdit }: Props) {
  const list = games(store.profiles);
  const base = defaultProfile(store.profiles);
  const active = store.activeProfile;
  const [selectedId, setSelectedId] = useState(() => (active?.game ? active.id : (list[0]?.id ?? DEFAULT_ID)));
  const [adding, setAdding] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const selected = store.profiles.find((p) => p.id === selectedId) ?? base;
  const detecting = store.settings.gameAutoSwitch;

  const run = (task: Promise<unknown>) => {
    setError(null);
    setNote(null);
    task.catch((e) => setError(String(e)));
  };
  const select = (id: string) => {
    setSelectedId(id);
    setConfirmDelete(false);
    setNote(null);
  };
  const duplicate = async (p: Profile) => {
    const copy = await duplicateProfile(p.id);
    await store.refresh();
    setNote(`Copied to Profiles as “${copy.name}”.`);
  };
  const remove = async (p: Profile) => {
    await store.removeGame(p.id);
    select(DEFAULT_ID);
  };

  return (
    <div className="page games">
      <header className="page-head">
        <h1>Game profiles</h1>
        <span className="muted">Each game keeps its own key bindings</span>
      </header>
      <section className="games-body">
        <div className="games-list">
          <div className="games-list-head">
            <span className="kb-label">Game profiles · {list.length}</span>
            <button type="button" className="btn primary" onClick={() => setAdding(true)}>
              + Add game
            </button>
          </div>
          <label className="games-auto">
            <span className={"dot" + (detecting ? " ok" : "")} />
            <span className="grow">Auto-detect is {detecting ? "on" : "off"}</span>
            <input
              type="checkbox"
              role="switch"
              className="switch"
              aria-label="Detect games and switch automatically"
              checked={detecting}
              onChange={(e) => run(store.setGameAutoSwitch(null, e.target.checked))}
            />
          </label>
          {base && (
            <button
              type="button"
              className={"game-row game-row-main" + (selected?.id === base.id ? " selected" : "")}
              aria-pressed={selected?.id === base.id}
              onClick={() => select(base.id)}
            >
              <span className="tile tile-md tile-letter tile-neutral" aria-hidden="true">
                D
              </span>
              <span className="game-row-text">
                <span className="game-row-name">Default (desktop)</span>
                <span className="muted small">{bindingCount(Object.keys(base.bindings).length)} · used when no game runs</span>
              </span>
            </button>
          )}
          {list.map((p) => {
            const g = p.game!;
            const on = p.id === selected?.id;
            return (
              <div key={p.id} className={"game-row" + (on ? " selected" : "")}>
                <button type="button" className="game-row-main" aria-pressed={on} onClick={() => select(p.id)}>
                  <GameTile name={g.name} icon={g.icon} size="md" />
                  <span className="game-row-text">
                    <span className="game-row-name">
                      {p.name}
                      {store.liveGame === p.id && <span className="live">LIVE</span>}
                    </span>
                    <span className="muted small">
                      {bindingCount(Object.keys(p.bindings).length)} · {gameExe(g)}
                    </span>
                  </span>
                </button>
                <input
                  type="checkbox"
                  role="switch"
                  className="switch"
                  aria-label={`Switch to ${g.name} automatically`}
                  checked={g.autoSwitch}
                  onChange={(e) => run(store.setGameAutoSwitch(p.id, e.target.checked))}
                />
              </div>
            );
          })}
          {list.length === 0 && <p className="muted small games-empty">No games yet. Add one that is running or installed.</p>}
        </div>

        {selected && (
          <div className="games-detail">
            {selected.game ? (
              <header className="gd-head">
                <GameTile name={selected.game.name} icon={selected.game.icon} size="xl" />
                <div className="col grow gap-6">
                  <h2>{selected.name}</h2>
                  <code className="gd-place">{gamePlace(selected.game)}</code>
                </div>
                <button type="button" className="btn" onClick={() => run(duplicate(selected))}>
                  Duplicate
                </button>
                <button
                  type="button"
                  className={"btn" + (confirmDelete ? " danger" : "")}
                  onClick={() => (confirmDelete ? run(remove(selected)) : setConfirmDelete(true))}
                >
                  {confirmDelete ? "Confirm delete" : "Delete"}
                </button>
              </header>
            ) : (
              <header className="gd-head">
                <span className="tile tile-xl tile-letter tile-neutral" aria-hidden="true">
                  D
                </span>
                <div className="col grow gap-6">
                  <h2>Default (desktop)</h2>
                  <span className="muted">Used when no game runs. Games use these keys unless they change them.</span>
                </div>
                <button type="button" className="btn" onClick={() => onEdit(selected.id)}>
                  Edit keys
                </button>
              </header>
            )}
            {(error || note) && <p className={error ? "error small" : "muted small"}>{error ?? note}</p>}

            {selected.game && (
              <label className="gd-auto">
                <span className="col grow gap-3">
                  <span className="strong">Switch automatically</span>
                  <span className="muted">
                    Apply these bindings when {gameExe(selected.game)} is in focus; go back to Default when it closes.
                  </span>
                </span>
                <input
                  type="checkbox"
                  role="switch"
                  className="switch"
                  aria-label={`Switch to ${selected.game.name} automatically`}
                  checked={selected.game.autoSwitch}
                  onChange={(e) => run(store.setGameAutoSwitch(selected.id, e.target.checked))}
                />
              </label>
            )}

            <div className="gd-keys-head">
              <span className="kb-label">
                {selected.game ? "Key bindings for this game" : "Key bindings"} · {Object.keys(selected.bindings).length}
              </span>
              <span className="grow" />
              {selected.game && base && (
                <button type="button" className="btn" onClick={() => run(store.saveProfile({ id: selected.id, bindings: copyFromDefault(selected, base) }))}>
                  Copy from Default
                </button>
              )}
              <button type="button" className="btn primary" onClick={() => onEdit(selected.id)}>
                + Add binding
              </button>
            </div>
            <ul className="gd-keys">
              {Object.entries(selected.bindings).map(([code, b]) => (
                <li key={code} className="gd-key">
                  <span className="gd-cap">
                    <Keycap code={code} size="sm" />
                  </span>
                  <span className="gd-arrow" aria-hidden="true">
                    →
                  </span>
                  <span className="gd-what">
                    {b.action.type === "block" ? (selected.game ? "Disabled in game" : "Disabled") : actionLabel(b.action, store.catalog, store.profiles)}
                    {b.mods.length > 0 && <span className="muted small"> · with {formatChord(b.mods)}</span>}
                  </span>
                  <span className={"chip" + (b.action.type === "block" ? " chip-off" : "")}>{chipOf(b.action)}</span>
                  <button type="button" className="icon-btn" aria-label={`Change ${code}`} title="Change" onClick={() => onEdit(selected.id, code)}>
                    ⋯
                  </button>
                </li>
              ))}
              {Object.keys(selected.bindings).length === 0 && (
                <li className="muted small gd-none">
                  {selected.game ? "No keys of its own yet: every key works as in your Default profile." : "No keys remapped."}
                </li>
              )}
            </ul>
            {selected.game && <p className="muted small">Keys not listed here fall back to your Default profile.</p>}
          </div>
        )}
      </section>
      {adding && (
        <AddGame
          catalog={store.catalog}
          profiles={store.profiles}
          onAdd={async (entry, b) => {
            const p = await store.addGame(entry, b);
            select(p.id);
            setAdding(false);
          }}
          onClose={() => setAdding(false)}
        />
      )}
    </div>
  );
}
