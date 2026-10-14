import { convertFileSrc } from "@tauri-apps/api/core";
import { entryGroup, isMac, type Group } from "../lib/actions";
import type { AppEntry } from "../lib/types";

const TITLE: Record<Group, string> = {
  builtin: isMac ? "macOS apps" : "Windows apps",
  installed: "Installed programs",
  steam: "Steam games",
};
const ORDER: Group[] = ["builtin", "installed", "steam"];

export type ProgramListProps = {
  entries: AppEntry[];
  query: string;
  /** Launch string of the program the key opens now. */
  selected: string | null;
  busy: boolean;
  onLaunch: (e: AppEntry) => void;
  onRefresh: () => void;
};

/** The key picker's "Launch program" tab: every detected program with its icon, filtered by name. */
export default function ProgramList({ entries, query, selected, busy, onLaunch, onRefresh }: ProgramListProps) {
  const q = query.trim().toLowerCase();
  const shown = entries.filter((e) => q === "" || e.name.toLowerCase().includes(q));
  return (
    <>
      <div className="kp-apps-head">
        <span className="muted small">
          {busy ? "Looking for programs…" : `${entries.length} programs found on this computer`}
        </span>
        <button type="button" className="kp-refresh" disabled={busy} onClick={onRefresh}>
          {busy ? "Scanning…" : "Refresh"}
        </button>
      </div>
      <div className="kp-list">
        {ORDER.map((g) => {
          const items = shown.filter((e) => entryGroup(e) === g);
          if (items.length === 0) return null;
          return (
            <section key={g}>
              <h3 className="kp-group">
                {TITLE[g]} <span className="muted">· {items.length}</span>
              </h3>
              <div className="kp-apps">
                {items.map((e) => (
                  <button
                    key={e.id}
                    type="button"
                    className="kp-app"
                    data-launch={e.launch}
                    aria-label={`Launch ${e.name}`}
                    aria-pressed={e.launch === selected}
                    title={e.launch}
                    onClick={() => onLaunch(e)}
                  >
                    <AppIcon entry={e} />
                    <span className="kp-app-name">{e.name}</span>
                  </button>
                ))}
              </div>
            </section>
          );
        })}
        {shown.length === 0 && (
          <p className="muted">{entries.length === 0 ? "No programs found yet. Press Refresh." : `No program matches “${query}”.`}</p>
        )}
      </div>
    </>
  );
}

/** The program's real icon (cached by the catalog scan), or its first letters until one exists. */
export function AppIcon({ entry }: { entry: AppEntry }) {
  return entry.icon ? (
    <img className="kp-app-icon" src={convertFileSrc(entry.icon)} alt="" loading="lazy" />
  ) : (
    <span className="kp-app-icon kp-glyph" aria-hidden="true">
      {entry.name.slice(0, 2)}
    </span>
  );
}
