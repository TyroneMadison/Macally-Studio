import { useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { initials, ruleFor } from "../lib/profile";
import type { AppEntry } from "../lib/types";

type Props = {
  catalog: AppEntry[];
  onPick: (rule: string) => void;
  onClose: () => void;
};

/** Pick an app to auto-activate a profile: catalog apps with a matchable exe / bundle id, or typed text. */
export default function AppRulePicker({ catalog, onPick, onClose }: Props) {
  const [query, setQuery] = useState("");
  const typed = query.trim();
  const needle = typed.toLowerCase();
  const rows = catalog.flatMap((app) => {
    const rule = ruleFor(app);
    if (!rule) return [];
    const hit = !needle || app.name.toLowerCase().includes(needle) || rule.toLowerCase().includes(needle);
    return hit ? [{ app, rule }] : [];
  });

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal" role="dialog" aria-modal="true" aria-label="Add app rule" onClick={(e) => e.stopPropagation()}>
        <div className="row gap-8">
          <h2 className="grow">Activate when this app is in front</h2>
          <button type="button" className="icon-btn" aria-label="Close" onClick={onClose}>
            ×
          </button>
        </div>
        <input
          autoFocus
          className="input"
          placeholder="Search apps, or type an exe name / bundle id"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => e.key === "Escape" && onClose()}
        />
        <ul className="app-list">
          {typed && (
            <li>
              <button type="button" className="app-row" onClick={() => onPick(typed)}>
                <span className="glyph xs">+</span>
                <span className="grow">Use “{typed}”</span>
              </button>
            </li>
          )}
          {rows.map(({ app, rule }) => (
            <li key={app.id}>
              <button type="button" className="app-row" onClick={() => onPick(rule)}>
                {app.icon ? (
                  <img src={convertFileSrc(app.icon)} alt="" width={22} height={22} />
                ) : (
                  <span className="glyph xs">{initials(app.name)}</span>
                )}
                <span className="grow">{app.name}</span>
                <code className="muted small">{rule}</code>
              </button>
            </li>
          ))}
          {!typed && rows.length === 0 && (
            <li className="muted small">No apps in the catalog yet. Type an exe name or bundle id above.</li>
          )}
        </ul>
      </div>
    </div>
  );
}
