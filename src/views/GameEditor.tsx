import { useState } from "react";
import GameTile from "../components/GameTile";
import Inspector from "../components/Inspector";
import Keycap from "../components/Keycap";
import KeyGrid from "../components/KeyGrid";
import { capNote, defaultProfile, gameExe } from "../lib/games";
import { initials } from "../lib/profile";
import type { AppState } from "../lib/state";

export type GameEditorProps = {
  store: AppState;
  profileId: string;
  /** Opened because the game was detected (not picked by hand). */
  auto: boolean;
  /** A key to start on. */
  initialKey?: string;
  /** Back to where you were. */
  onBack: () => void;
  /** To Game profiles. */
  onProfiles: () => void;
  /** Close, to the Keyboard view. */
  onExit: () => void;
};

/** One profile's keys, full screen: opens when a game's profile switches in, by itself or by hand. */
export default function GameEditor({ store, profileId, auto, initialKey, onBack, onProfiles, onExit }: GameEditorProps) {
  const [selected, setSelected] = useState<string | null>(initialKey ?? null);
  const p = store.profiles.find((x) => x.id === profileId);
  if (!p) return null;
  const game = p.game;
  const base = game ? defaultProfile(store.profiles) : null;
  const active = store.activeProfile?.id === p.id;
  const legends = store.settings.windowsLegends ? "win" : "mac";
  const note = (b: Parameters<typeof capNote>[0]) => capNote(b, store.catalog, store.profiles);
  const remaps = Object.entries(p.bindings);

  return (
    <div className="ge">
      <header className="ge-head">
        <button type="button" className="btn ge-btn" onClick={onBack}>
          ← Back
        </button>
        {game ? (
          <GameTile name={game.name} icon={game.icon} size="md" />
        ) : (
          <span className="tile tile-md tile-letter tile-neutral" aria-hidden="true">
            {initials(p.name)}
          </span>
        )}
        <div className="ge-title">
          <span className="ge-name">{p.name}</span>
          <code className="ge-exe">{game ? gameExe(game) : "Used when no game runs"}</code>
        </div>
        {active && (
          <span className="ge-active">
            <span className="dot ok" />
            Active for {game ? game.name : "the desktop"}
          </span>
        )}
        <span className="grow" />
        {game && (
          <label className="ge-auto">
            Auto-switch
            <input
              type="checkbox"
              role="switch"
              className="switch"
              aria-label={`Switch to ${game.name} automatically`}
              checked={game.autoSwitch}
              onChange={(e) => void store.setGameAutoSwitch(p.id, e.target.checked)}
            />
          </label>
        )}
        <button type="button" className="btn ge-btn" onClick={onProfiles}>
          Profiles
        </button>
        <button type="button" className="btn ge-btn" onClick={onExit}>
          ✕ Exit
        </button>
      </header>
      <section className="ge-body">
        <div className="ge-board">
          <div className="ge-board-head">
            <span className="kb-label">Click any key to change it{game && ` · ${game.name} only`}</span>
            <span className="grow" />
            <span className="ge-legend">
              <i className="sw remapped" />
              Remapped
              <i className="sw off" />
              Disabled
            </span>
          </div>
          <KeyGrid layout={store.layout} bindings={p.bindings} legends={legends} selected={selected} onSelect={setSelected} note={note} />
          {game && (
            <div className="ge-note">
              <span className="dot ok" />
              {auto ? `Opened automatically when ${game.name} started.` : `Opened when you switched to ${game.name}.`} Unchanged keys use your
              Default profile.
            </div>
          )}
        </div>
        <div className="ge-side">
          <Inspector
            keyCode={selected}
            layout={store.layout}
            profile={p}
            saveBinding={store.saveBinding}
            catalog={store.catalog}
            profiles={store.profiles}
            lastKey={store.lastKey}
            fallback={base ? (code) => base.bindings[code] : undefined}
            resetKeyEverywhere={store.resetKeyEverywhere}
            onSelectKey={setSelected}
          />
          <div className="ge-remaps">
            <div className="kb-label">Remaps in this profile · {remaps.length}</div>
            <ul>
              {remaps.map(([code, b]) => (
                <li key={code}>
                  <button type="button" className={"ge-remap" + (code === selected ? " on" : "")} onClick={() => setSelected(code)}>
                    <Keycap code={code} size="sm" />
                    <span className="gd-arrow" aria-hidden="true">
                      →
                    </span>
                    <span className={b.action.type === "block" ? "ge-off" : ""}>{note(b)}</span>
                  </button>
                </li>
              ))}
            </ul>
          </div>
          <button type="button" className="btn primary ge-done" onClick={onBack}>
            Done
          </button>
        </div>
      </section>
    </div>
  );
}
