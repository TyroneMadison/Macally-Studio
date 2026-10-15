import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import GameTile from "./GameTile";
import { isMac } from "../lib/actions";
import { DEFAULT_ID, entryExe } from "../lib/games";
import { getRunningApps } from "../lib/ipc";
import type { AppEntry, Profile } from "../lib/types";

export type AddGameProps = {
  catalog: AppEntry[];
  profiles: Profile[];
  /** Running apps to start with; null fetches them (get_running_apps). */
  running?: AppEntry[] | null;
  /** Adds the game; `base` is a profile to copy keys from, null starts on top of the Default. */
  onAdd: (entry: AppEntry, base: string | null) => Promise<void>;
  onClose: () => void;
};

type Tab = "running" | "installed";

/** A program picked with Browse, as the catalog would list it. */
function browsed(path: string): AppEntry {
  const name = path.split(/[\\/]/).pop()?.replace(/\.(exe|app)$/i, "") ?? path;
  return { id: `app:${path}`, name, kind: "path", launch: path, icon: null, source: isMac ? "macos" : "windows" };
}

/** "Add a game": a running process, an installed program or a browsed .exe gets its own profile. */
export default function AddGame({ catalog, profiles, running: given = null, onAdd, onClose }: AddGameProps) {
  const [running, setRunning] = useState<AppEntry[] | null>(given);
  const [tab, setTab] = useState<Tab>("running");
  const [query, setQuery] = useState("");
  const [picked, setPicked] = useState<AppEntry | null>(null);
  const [base, setBase] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const dialog = useRef<HTMLDivElement>(null);

  useEffect(() => {
    dialog.current?.focus();
    if (given !== null) return;
    getRunningApps().then(setRunning, (e) => {
      setRunning([]);
      setError(String(e));
    });
  }, []); // once per open

  const q = query.trim().toLowerCase();
  const match = (e: AppEntry) => q === "" || e.name.toLowerCase().includes(q) || entryExe(e).toLowerCase().includes(q);
  const steam = catalog.filter((e) => e.kind === "steam");
  const bases = profiles.filter((p) => !p.game && p.id !== DEFAULT_ID);

  async function add(e: AppEntry) {
    setBusy(true);
    setError(null);
    try {
      await onAdd(e, base || null);
    } catch (err) {
      setError(String(err));
      setBusy(false);
    }
  }
  async function browse() {
    const path = await open({ multiple: false, filters: [{ name: "Program", extensions: isMac ? ["app"] : ["exe"] }] });
    if (typeof path === "string") await add(browsed(path));
  }

  const section = (title: string, items: AppEntry[] | null, live = false) => (
    <section className="ag-section">
      <h3 className="kb-label">
        {title} · {items?.length ?? 0}
      </h3>
      {items === null ? (
        <p className="muted small">Looking for running programs…</p>
      ) : items.length === 0 ? (
        <p className="muted small">{q ? `Nothing matches “${query}”.` : "None found."}</p>
      ) : (
        items.map((e) => (
          <button
            key={e.id}
            type="button"
            className={"ag-row" + (picked?.id === e.id ? " on" : "")}
            aria-pressed={picked?.id === e.id}
            title={e.launch}
            onClick={() => setPicked(e)}
            onDoubleClick={() => void add(e)}
          >
            <GameTile name={e.name} icon={e.icon} size="sm" />
            <span className="ag-name">{e.name}</span>
            {live && <span className="dot ok" />}
            <code className="ag-exe">{entryExe(e)}</code>
          </button>
        ))
      )}
    </section>
  );

  return (
    <div className="ap-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div
        className="ag"
        role="dialog"
        aria-modal="true"
        aria-labelledby="ag-title"
        tabIndex={-1}
        ref={dialog}
        onKeyDown={(e) => e.key === "Escape" && onClose()}
      >
        <header className="ag-head">
          <h2 id="ag-title">Add a game</h2>
          <button type="button" className="icon-btn" aria-label="Close" onClick={onClose}>
            ×
          </button>
        </header>
        <p className="muted">Pick a running process or an installed program. Its profile switches on automatically when it starts.</p>
        <div className="seg ag-tabs" role="tablist" aria-label="Where the game comes from">
          <button type="button" role="tab" aria-selected={tab === "running"} onClick={() => setTab("running")}>
            Running · {running?.length ?? 0}
          </button>
          <button type="button" role="tab" aria-selected={tab === "installed"} onClick={() => setTab("installed")}>
            Installed · {catalog.length}
          </button>
          <button type="button" onClick={() => void browse()}>
            {isMac ? "Browse .app…" : "Browse .exe…"}
          </button>
        </div>
        <input
          type="search"
          className="input"
          aria-label="Search programs and processes"
          placeholder="Search programs and processes"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <div className="ag-list">
          {tab === "running" ? (
            <>
              {section("Running now", running && running.filter(match), true)}
              {section("Installed games", steam.filter(match))}
            </>
          ) : (
            section("Installed programs", catalog.filter(match))
          )}
        </div>
        {error && (
          <p className="error small" role="alert">
            {error}
          </p>
        )}
        <footer className="ag-foot">
          <label className="ag-base">
            Start from:
            <select className="ag-select" value={base} onChange={(e) => setBase(e.target.value)}>
              <option value="">Default profile</option>
              {bases.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
          </label>
          <span className="grow" />
          <button type="button" className="btn" onClick={onClose}>
            Cancel
          </button>
          <button type="button" className="btn primary" disabled={!picked || busy} onClick={() => picked && void add(picked)}>
            {picked ? `Add ${picked.name}` : "Add game"}
          </button>
        </footer>
      </div>
    </div>
  );
}
