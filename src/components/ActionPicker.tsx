import { useEffect, useRef, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import AppGrid from "./AppGrid";
import ChordField from "./ChordField";
import {
  describeAction,
  entryToAction,
  formatChord,
  isComplete,
  isMac,
  joinArgs,
  launchOf,
  splitArgs,
  type Group,
} from "../lib/actions";
import { refreshCatalog } from "../lib/ipc";
import type { Action, AppEntry, Profile } from "../lib/types";

export type ActionPickerProps = {
  open: boolean;
  keyCode: string;
  initial?: Action;
  catalog: AppEntry[];
  profiles: Profile[];
  onPick: (a: Action) => void;
  onClose: () => void;
};

type Tab = "key" | "app" | "steam" | "open" | "command" | "profile" | "pass";
type Filter = "all" | "builtin" | "installed";

const TABS: { id: Tab; label: string; icon: string }[] = [
  { id: "key", label: "Key / Combo", icon: "M3 7h18v10H3zM7 11h.01M11 11h.01M15 11h.01M8 14h8" },
  { id: "app", label: "App", icon: "M3 3h7v7H3zM14 3h7v7h-7zM3 14h7v7H3zM14 14h7v7h-7z" },
  { id: "steam", label: "Steam game", icon: "M6 12a6 6 0 0 1 12 0v3a3 3 0 0 1-3 3H9a3 3 0 0 1-3-3zM9 12h2M10 11v2M14 12h.01M16 12h.01" },
  { id: "open", label: "URL / File", icon: "M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1.5 1.5M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1.5-1.5" },
  { id: "command", label: "Command", icon: "M4 17l6-5-6-5M12 19h8" },
  { id: "profile", label: "Switch profile", icon: "M12 3l9 5-9 5-9-5 9-5zM3 13l9 5 9-5" },
  { id: "pass", label: "Pass-through", icon: "M4 7h16M4 12h16M4 17h10" },
];

const FILTERS: { id: Filter; label: string }[] = [
  { id: "all", label: "All" },
  { id: "builtin", label: "Built-in" },
  { id: "installed", label: "Installed" },
];

function tabFor(a?: Action): Tab {
  switch (a?.type) {
    case "launch_app":
      return "app";
    case "steam_game":
      return "steam";
    case "open":
      return "open";
    case "command":
      return "command";
    case "switch_profile":
    case "pause_toggle":
      return "profile";
    case "pass_through":
      return "pass";
    default:
      return "key";
  }
}

function Icon({ d }: { d: string }) {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d={d} />
    </svg>
  );
}

export default function ActionPicker(props: ActionPickerProps) {
  // Mounted only while open, so every open starts fresh from `initial`.
  return props.open ? <PickerDialog {...props} /> : null;
}

function PickerDialog({ keyCode, initial, catalog, profiles, onPick, onClose }: ActionPickerProps) {
  const start = initial && isComplete(initial) ? initial : undefined;
  const [tab, setTab] = useState<Tab>(tabFor(initial));
  const [keys, setKeys] = useState<string[]>(start?.type === "key_combo" ? start.keys : []);
  const [picked, setPicked] = useState<Action | null>(
    start?.type === "launch_app" || start?.type === "steam_game" ? start : null,
  );
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<Filter>("all");
  const [fresh, setFresh] = useState<AppEntry[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [target, setTarget] = useState(start?.type === "open" ? start.target : "");
  const [program, setProgram] = useState(start?.type === "command" ? start.program : "");
  const [args, setArgs] = useState(start?.type === "command" ? joinArgs(start.args) : "");
  const [hidden, setHidden] = useState(start?.type === "command" ? start.hidden : true);
  const [special, setSpecial] = useState<Action | null>(
    start?.type === "switch_profile" || start?.type === "pause_toggle" ? start : null,
  );
  const dialog = useRef<HTMLDivElement>(null);
  const list = fresh ?? catalog;
  const keyLabel = formatChord([keyCode]);

  useEffect(() => dialog.current?.focus(), []);
  // First run: the launch-time scan may still be going; wait for a full list instead of showing none.
  useEffect(() => {
    if (catalog.length === 0) void refresh();
  }, []); // once per open

  const current: Action | null =
    tab === "key"
      ? { type: "key_combo", keys }
      : tab === "app" || tab === "steam"
        ? picked
        : tab === "open"
          ? { type: "open", target: target.trim() }
          : tab === "command"
            ? { type: "command", program: program.trim(), args: splitArgs(args), hidden }
            : tab === "profile"
              ? special
              : { type: "pass_through" };
  const ready = current !== null && isComplete(current);
  const summary = ready ? describeAction(current, list, profiles) : null;
  const groups: Group[] = tab === "steam" ? ["steam"] : filter === "all" ? ["builtin", "installed", "steam"] : [filter];

  async function refresh() {
    setBusy(true);
    setError(null);
    try {
      setFresh(await refreshCatalog());
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function browse(kind: "app" | "file" | "program") {
    setError(null);
    try {
      const filters =
        kind === "app" ? [{ name: "Programs", extensions: isMac ? ["app"] : ["exe", "lnk", "bat", "cmd"] }] : undefined;
      const path = await openDialog({ multiple: false, directory: false, filters });
      if (typeof path !== "string") return;
      if (kind === "app") setPicked({ type: "launch_app", app: { kind: "path", path, args: "" } });
      else if (kind === "file") setTarget(path);
      else setProgram(path);
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div className="ap-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div
        ref={dialog}
        className="ap"
        role="dialog"
        aria-modal="true"
        aria-labelledby="ap-title"
        tabIndex={-1}
        onKeyDown={(e) => e.key === "Escape" && onClose()}
      >
        <header className="ap-head">
          <span className="ap-keycap">{keyLabel}</span>
          <h2 id="ap-title">Choose what {keyLabel} does</h2>
          <button type="button" className="ap-icon-btn" aria-label="Close" onClick={onClose}>
            <Icon d="M6 6l12 12M18 6L6 18" />
          </button>
        </header>
        <div className="ap-body">
          <div className="ap-tabs" role="tablist" aria-label="Action type" aria-orientation="vertical">
            {TABS.map((t) => (
              <button
                key={t.id}
                type="button"
                role="tab"
                id={`ap-tab-${t.id}`}
                aria-selected={tab === t.id}
                aria-controls="ap-panel"
                className={"ap-tab" + (t.id === "pass" ? " ap-tab-last" : "")}
                onClick={() => setTab(t.id)}
              >
                <Icon d={t.icon} />
                {t.label}
              </button>
            ))}
          </div>
          <div className="ap-panel" id="ap-panel" role="tabpanel" aria-labelledby={`ap-tab-${tab}`}>
            {tab === "key" && (
              <section className="ap-pane">
                <label className="kb-label" htmlFor="ap-chord">Send keys</label>
                <ChordField id="ap-chord" initial={keys} onChange={setKeys} />
                <p className="muted">
                  Join key names with + (Ctrl, Alt, Shift, {isMac ? "Cmd" : "Win"}, A–Z, 0–9, F1–F15, Tab, Esc…) or
                  click Record and press the keys.
                </p>
              </section>
            )}

            {(tab === "app" || tab === "steam") && (
              <>
                <div className="ap-tools">
                  <input
                    type="search"
                    className="input"
                    aria-label="Search apps"
                    placeholder={`Search ${list.length} apps`}
                    value={query}
                    onChange={(e) => setQuery(e.target.value)}
                  />
                  {tab === "app" && (
                    <div className="seg" role="group" aria-label="Filter">
                      {FILTERS.map((f) => (
                        <button key={f.id} type="button" aria-pressed={filter === f.id} onClick={() => setFilter(f.id)}>
                          {f.label}
                        </button>
                      ))}
                    </div>
                  )}
                  <button type="button" className="ap-icon-btn" aria-label="Refresh app list" disabled={busy} onClick={() => void refresh()}>
                    <Icon d="M21 12a9 9 0 1 1-2.6-6.4M21 3v6h-6" />
                  </button>
                </div>
                <AppGrid
                  entries={list}
                  groups={groups}
                  query={query}
                  selected={launchOf(picked)}
                  busy={busy}
                  onSelect={(e) => setPicked(entryToAction(e))}
                  onBind={(e) => onPick(entryToAction(e))}
                />
              </>
            )}

            {tab === "open" && (
              <section className="ap-pane">
                <label className="kb-label" htmlFor="ap-target">URL, file or folder</label>
                <div className="chord">
                  <input
                    id="ap-target"
                    className="input mono"
                    value={target}
                    placeholder="https://github.com"
                    onChange={(e) => setTarget(e.target.value)}
                  />
                  <button type="button" className="btn" onClick={() => void browse("file")}>Browse…</button>
                </div>
                <p className="muted">Opens in the default browser, or the default app for that file or folder.</p>
              </section>
            )}

            {tab === "command" && (
              <section className="ap-pane">
                <label className="kb-label" htmlFor="ap-program">Program</label>
                <div className="chord">
                  <input
                    id="ap-program"
                    className="input mono"
                    value={program}
                    placeholder={isMac ? "/usr/bin/say" : "powershell"}
                    onChange={(e) => setProgram(e.target.value)}
                  />
                  <button type="button" className="btn" onClick={() => void browse("program")}>Browse…</button>
                </div>
                <label className="kb-label" htmlFor="ap-args">Arguments</label>
                <input
                  id="ap-args"
                  className="input mono"
                  value={args}
                  placeholder={isMac ? "Hello" : '-File "C:\\scripts\\run.ps1"'}
                  onChange={(e) => setArgs(e.target.value)}
                />
                <label className="ap-check">
                  <input type="checkbox" checked={hidden} onChange={(e) => setHidden(e.target.checked)} />
                  Run hidden (no console window)
                </label>
              </section>
            )}

            {tab === "profile" && (
              <section className="ap-pane">
                <div className="kb-label" id="ap-profiles">Switch to profile</div>
                <div className="ap-list" role="radiogroup" aria-labelledby="ap-profiles">
                  {profiles.map((p) => {
                    const on = special?.type === "switch_profile" && special.profile === p.id;
                    return (
                      <button
                        key={p.id}
                        type="button"
                        role="radio"
                        aria-checked={on}
                        className={"ap-item" + (on ? " on" : "")}
                        onClick={() => setSpecial({ type: "switch_profile", profile: p.id })}
                      >
                        {p.name}
                        {p.builtin && <span className="muted"> · built-in</span>}
                      </button>
                    );
                  })}
                  <button
                    type="button"
                    role="radio"
                    aria-checked={special?.type === "pause_toggle"}
                    className={"ap-item" + (special?.type === "pause_toggle" ? " on" : "")}
                    onClick={() => setSpecial({ type: "pause_toggle" })}
                  >
                    Pause / resume remapping
                  </button>
                </div>
              </section>
            )}

            {tab === "pass" && (
              <section className="ap-pane">
                <p className="muted">{keyLabel} keeps sending its own key in this profile. Nothing is remapped.</p>
              </section>
            )}

            <footer className="ap-foot">
              <div className="ap-sel">
                {error ? (
                  <span className="ap-error" role="alert">{error}</span>
                ) : summary ? (
                  <>
                    <span className="muted">Selected:</span>
                    <b>{summary.title}</b>
                    <code>{summary.detail}</code>
                  </>
                ) : (
                  <span className="muted">Nothing selected yet</span>
                )}
              </div>
              {tab === "app" && (
                <button type="button" className="btn" onClick={() => void browse("app")}>
                  {isMac ? "Browse for .app" : "Browse for .exe"}
                </button>
              )}
              <button type="button" className="btn primary" disabled={!ready} onClick={() => current && onPick(current)}>
                Bind to {keyLabel}
              </button>
            </footer>
          </div>
        </div>
      </div>
    </div>
  );
}
