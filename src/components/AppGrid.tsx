import { convertFileSrc } from "@tauri-apps/api/core";
import { entryGroup, isMac, type Group } from "../lib/actions";
import type { AppEntry } from "../lib/types";

const TITLE: Record<Group, string> = {
  builtin: isMac ? "macOS built-in" : "Windows built-in",
  installed: "Installed apps",
  steam: "Steam library",
};

export type AppGridProps = {
  entries: AppEntry[];
  groups: Group[];
  query: string;
  selected: string | null; // launch string of the picked entry
  busy: boolean;
  onSelect: (e: AppEntry) => void;
  onBind: (e: AppEntry) => void;
};

/** The App and Steam tabs: one titled section of icon tiles per group, filtered by name. */
export default function AppGrid({ entries, groups, query, selected, busy, onSelect, onBind }: AppGridProps) {
  const q = query.trim().toLowerCase();
  return (
    <div className="ap-scroll">
      {groups.map((g) => {
        const items = entries.filter((e) => entryGroup(e) === g && (q === "" || e.name.toLowerCase().includes(q)));
        return (
          <section key={g} className="ap-section" aria-label={TITLE[g]}>
            <h3 className="kb-label">
              {TITLE[g]} <span className="muted">· {items.length}</span>
            </h3>
            {items.length === 0 ? (
              <p className="muted">
                {busy ? "Scanning…" : g === "steam" ? "No installed Steam games found." : "No matching apps."}
              </p>
            ) : (
              <div className="ap-grid">
                {items.map((e) => (
                  <button
                    key={e.id}
                    type="button"
                    className={"ap-tile" + (e.launch === selected ? " on" : "")}
                    aria-pressed={e.launch === selected}
                    title={e.name}
                    onClick={() => onSelect(e)}
                    onDoubleClick={() => onBind(e)}
                  >
                    {e.icon ? (
                      <img src={convertFileSrc(e.icon)} alt="" loading="lazy" />
                    ) : (
                      <span className="ap-glyph" aria-hidden="true">
                        {e.name.slice(0, 2)}
                      </span>
                    )}
                    <span className="ap-name">{e.name}</span>
                  </button>
                ))}
              </div>
            )}
          </section>
        );
      })}
    </div>
  );
}
