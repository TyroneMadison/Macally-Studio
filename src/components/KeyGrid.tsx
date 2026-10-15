import { useMemo } from "react";
import { isFirmwareOnly } from "../lib/keys";
import type { Binding, Layout } from "../lib/types";

export type KeyGridProps = {
  layout: Layout;
  /** The profile's own bindings: those keys are marked, blue when remapped, red when off. */
  bindings: Record<string, Binding>;
  legends: "mac" | "win";
  selected: string | null;
  onSelect: (code: string) => void;
  /** What a bound key does, short enough for its cap. */
  note: (b: Binding) => string;
};

const GAP = 0.1; // between caps, in key units, like the 3D keyboard
/** Mac legends too long for a flat cap. */
const SHORT: Record<string, string> = { command: "cmd", option: "opt", control: "ctrl", "page up": "pg up", "page down": "pg dn" };
/** Top edge of a row: the F row is 0.6u tall, then a little air (Keyboard3D's rowTop). */
const rowTop = (row: number) => (row === 0 ? 0 : 0.75 + (row - 1));

/** The whole keyboard flat, every cap clickable: the full-screen editor's board. */
export default function KeyGrid({ layout, bindings, legends, selected, onSelect, note }: KeyGridProps) {
  const [width, depth] = useMemo(
    () => [Math.max(...layout.keys.map((k) => k.x + k.w)), Math.max(...layout.keys.map((k) => rowTop(k.row) + k.h))],
    [layout],
  );
  return (
    <div className="kg" style={{ aspectRatio: `${width} / ${depth}` }}>
      {layout.keys.map((k) => {
        const b = bindings[k.code];
        const printed = legends === "mac" ? k.mac : k.win;
        const legend = SHORT[printed] ?? (printed || (k.code === "Space" ? "" : k.code));
        const what = b && note(b);
        const cls = ["kg-key", isFirmwareOnly(k.code) ? "firmware" : b ? (b.action.type === "block" ? "off" : "remapped") : "", k.code === selected ? "selected" : ""]
          .filter(Boolean)
          .join(" ");
        return (
          <button
            key={k.code}
            type="button"
            className={cls}
            data-code={k.code}
            aria-label={what ? `${legend || k.code}: ${what}` : legend || k.code}
            aria-pressed={k.code === selected}
            style={{
              left: `${(k.x / width) * 100}%`,
              top: `${(rowTop(k.row) / depth) * 100}%`,
              width: `${((k.w - GAP) / width) * 100}%`,
              height: `${((k.h - GAP) / depth) * 100}%`,
            }}
            onClick={() => onSelect(k.code)}
          >
            <span className="kg-leg">{legend}</span>
            {what && <span className="kg-note">{what}</span>}
          </button>
        );
      })}
    </div>
  );
}
