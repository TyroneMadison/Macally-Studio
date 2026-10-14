import { useEffect, useLayoutEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import Keycap, { ChordCaps } from "./Keycap";
import { formatChord, isMac, KEY_GROUPS, keyLabel, type Mod } from "../lib/actions";
import { buildChord, capFace, keyMatches, WINDOWS_SHORTCUTS } from "../lib/keycap";

export type KeyPickerProps = {
  /** The chord the key sends now (empty when it sends nothing of its own). */
  value: string[];
  onPick: (keys: string[]) => void;
  mac?: boolean;
  /** Opens on first render (tests). */
  defaultOpen?: boolean;
};

const MOD_CODE: Record<Mod, string> = { Meta: "MetaLeft", Ctrl: "ControlLeft", Alt: "AltLeft", Shift: "ShiftLeft" };
/** Win first: it is the one people come here for (Win + V, Win + C). */
const MOD_CHIPS: Mod[] = ["Meta", "Ctrl", "Alt", "Shift"];

function modName(m: Mod, mac: boolean): string {
  return formatChord([m], mac);
}

/** "Remap to" list: every key as a keycap thumbnail, "send with" modifiers, and Windows shortcuts. */
export default function KeyPicker({ value, onPick, mac = isMac, defaultOpen = false }: KeyPickerProps) {
  const [open, setOpen] = useState(defaultOpen);
  const [query, setQuery] = useState("");
  const [mods, setMods] = useState<Mod[]>([]);
  const root = useRef<HTMLDivElement>(null);
  const [place, setPlace] = useState<CSSProperties>({});

  // fixed to the window, under the button, never taller than the room left below it
  useLayoutEffect(() => {
    const r = root.current?.getBoundingClientRect();
    if (!open || !r) return;
    const top = r.bottom + 6;
    setPlace({ top, right: Math.max(8, window.innerWidth - r.right), maxHeight: Math.max(240, window.innerHeight - top - 12) });
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const away = (e: MouseEvent) => {
      if (!root.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", away);
    return () => document.removeEventListener("mousedown", away);
  }, [open]);

  const groups = useMemo(
    () =>
      KEY_GROUPS.map((g) => ({ label: g.label, codes: g.codes.filter((c) => keyMatches(c, keyLabel(c, mac), query)) })).filter(
        (g) => g.codes.length > 0,
      ),
    [query, mac],
  );
  const shortcuts = mac
    ? []
    : WINDOWS_SHORTCUTS.filter((s) => keyMatches("", `${s.label} ${s.words} ${formatChord(s.keys, mac)}`, query));
  const single = value.length === 1 ? value[0] : null;

  const pick = (keys: string[]) => {
    onPick(keys);
    setOpen(false);
    setQuery("");
    setMods([]);
  };
  const toggle = (m: Mod) => setMods((ms) => (ms.includes(m) ? ms.filter((x) => x !== m) : [...ms, m]));

  return (
    <div
      className="kp"
      ref={root}
      onKeyDown={(e) => {
        if (e.key === "Escape" && open) {
          e.stopPropagation();
          setOpen(false);
        }
      }}
    >
      <button
        type="button"
        className="input kp-trigger"
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-label={value.length ? `Sends ${formatChord(value, mac)}. Choose a key` : "Choose a key"}
        onClick={() => setOpen((o) => !o)}
      >
        {value.length ? <ChordCaps keys={value} mac={mac} /> : <span className="muted">Choose a key…</span>}
        <svg className="kp-chevron" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M6 9l6 6 6-6" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      </button>

      {open && (
        <div className="kp-pop" role="dialog" aria-label="Choose a key" style={place}>
          <input
            type="search"
            className="input kp-search"
            placeholder="Search keys: win, clipboard, F13…"
            aria-label="Search keys"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            autoFocus
          />
          <div className="kp-mods" role="group" aria-label="Send together with">
            <span className="kp-label">Send with</span>
            {MOD_CHIPS.map((m) => (
              <button
                key={m}
                type="button"
                className="kp-mod"
                aria-pressed={mods.includes(m)}
                aria-label={`Send with ${modName(m, mac)}`}
                onClick={() => toggle(m)}
              >
                <Keycap code={MOD_CODE[m]} mac={mac} size="sm" />
                <span>{modName(m, mac)}</span>
              </button>
            ))}
          </div>
          {mods.length > 0 && (
            <p className="kp-hint">
              Pick a key to send {formatChord(buildChord(mods, "", mac).slice(0, -1), mac)} + that key.
            </p>
          )}

          <div className="kp-list">
            {shortcuts.length > 0 && (
              <section>
                <h3 className="kp-group">Windows shortcuts</h3>
                <div className="kp-shortcuts">
                  {shortcuts.map((s) => (
                    <button
                      key={s.label}
                      type="button"
                      className="kp-shortcut"
                      aria-label={`${s.label}: ${formatChord(s.keys, mac)}`}
                      onClick={() => pick(s.keys)}
                    >
                      <span>{s.label}</span>
                      <ChordCaps keys={s.keys} mac={mac} />
                    </button>
                  ))}
                </div>
              </section>
            )}
            {groups.map((g) => (
              <section key={g.label}>
                <h3 className="kp-group">{g.label}</h3>
                <div className="kp-grid">
                  {g.codes.map((c) => {
                    const face = capFace(c, mac);
                    const name = keyLabel(c, mac);
                    return (
                    <button
                      key={c}
                      type="button"
                      className={["kp-key", face.wide ? "wide" : "", c === single ? "on" : ""].filter(Boolean).join(" ")}
                      data-code={c}
                      aria-label={name}
                      aria-pressed={c === single}
                      title={name}
                      onClick={() => pick(buildChord(mods, c, mac))}
                    >
                      <Keycap code={c} mac={mac} lit={c === single} />
                      {/* the cap already says "A" or "F13"; name the rest ("Left Win", "Play/Pause") */}
                      {name.toLowerCase() !== face.legend.toLowerCase() && <span className="kp-name">{name}</span>}
                    </button>
                    );
                  })}
                </div>
              </section>
            ))}
            {shortcuts.length === 0 && groups.length === 0 && <p className="muted">No key matches “{query}”.</p>}
          </div>
        </div>
      )}
    </div>
  );
}
