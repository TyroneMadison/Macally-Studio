import { formatChord } from "./actions";
import type { Action, AppEntry, Binding, Profile } from "./types";

/** `base`, else `base-2`, `base-3`, ... (same rule as Store::free_id in Rust). */
export function uniqueId(base: string, taken: string[]): string {
  if (!taken.includes(base)) return base;
  for (let n = 2; ; n++) if (!taken.includes(`${base}-${n}`)) return `${base}-${n}`;
}

/** `p` with key `code` rebound to what `update` makes of its current binding (null unbinds it). */
export function withBinding(p: Profile, code: string, update: (old?: Binding) => Binding | null): Profile {
  const bindings = { ...p.bindings };
  const b = update(bindings[code]);
  if (b) bindings[code] = b;
  else delete bindings[code];
  return { ...p, bindings };
}

export function newProfile(existing: Profile[]): Profile {
  return {
    schema: 1,
    id: uniqueId("new-profile", existing.map((p) => p.id)),
    name: "New profile",
    builtin: false,
    hotkey: null,
    autoApps: [],
    bindings: {},
  };
}

/** Two-letter card glyph: "Photo editing" -> "PE", "Gaming" -> "GA". */
export function initials(name: string): string {
  const words = name.trim().split(/\s+/).filter(Boolean);
  const letters = words.length > 1 ? words[0][0] + words[1][0] : (words[0] ?? "?").slice(0, 2);
  return letters.toUpperCase();
}

const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;

/**
 * What foreground-app matching compares against: exe file name on Windows, bundle id on macOS.
 * Entries without one (Store apps, Steam games) give null; the picker lets you type the exe name.
 */
export function ruleFor(app: AppEntry): string | null {
  if (app.kind === "bundle") return app.launch;
  const file = fileName(app.launch);
  return /\.exe$/i.test(file) ? file : null;
}

/** One-line summary of an action for the bindings grid. */
export function actionLabel(a: Action, catalog: AppEntry[], profiles: Profile[]): string {
  switch (a.type) {
    case "key_combo":
      return formatChord(a.keys);
    case "launch_app": {
      const id = a.app.kind === "path" ? a.app.path : a.app.id;
      // the catalog stores Start-menu apps as shell:AppsFolder\<AUMID>
      const launch = a.app.kind === "aumid" ? `shell:AppsFolder\\${id}` : id;
      return catalog.find((e) => e.launch === launch)?.name ?? fileName(id);
    }
    case "steam_game":
      return a.name;
    case "open":
      return a.target;
    case "command":
      return [a.program, ...a.args].join(" ");
    case "switch_profile":
      return `Switch to ${profiles.find((p) => p.id === a.profile)?.name ?? a.profile}`;
    case "pass_through":
      return "Pass-through";
  }
}
