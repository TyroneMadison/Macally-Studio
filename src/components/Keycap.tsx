import { capFace, type CapIcon } from "../lib/keycap";

/** 24x24 icon paths, drawn in the legend colour. The Windows logo and the command loop are filled. */
const PATHS: Record<CapIcon, { d: string; fill?: boolean }> = {
  windows: { d: "M3 3h8.4v8.4H3zM12.6 3H21v8.4h-8.4zM3 12.6h8.4V21H3zM12.6 12.6H21V21h-8.4z", fill: true },
  command: {
    d: "M9 9V6.5A2.5 2.5 0 1 0 6.5 9H9zm0 0h6m-6 0v6m6-6V6.5A2.5 2.5 0 1 1 17.5 9H15zm0 0v6m0 0h2.5a2.5 2.5 0 1 1-2.5 2.5V15zm0 0H9m0 0v2.5A2.5 2.5 0 1 1 6.5 15H9z",
  },
  option: { d: "M3 6h5l7 12h6M14 6h7" },
  "play-pause": { d: "M4 6v12l8-6zM15 6v12M20 6v12" },
  prev: { d: "M5 6v12M19 6v12l-8-6zM12 6v12l-8-6" },
  next: { d: "M19 6v12M5 6v12l8-6zM12 6v12l8-6" },
  mute: { d: "M4 9h4l5-4v14l-5-4H4zM16 9l5 6M21 9l-5 6" },
  "vol-down": { d: "M4 9h4l5-4v14l-5-4H4zM17 9.5a3.5 3.5 0 0 1 0 5" },
  "vol-up": { d: "M4 9h4l5-4v14l-5-4H4zM16.5 9a4 4 0 0 1 0 6M19 6.5a7.5 7.5 0 0 1 0 11" },
  "brightness-down": { d: "M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM12 4v1.5M12 18.5V20M4 12h1.5M18.5 12H20" },
  "brightness-up": {
    d: "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8zM12 2v3M12 19v3M2 12h3M19 12h3M5 5l2 2M17 17l2 2M5 19l2-2M17 7l2-2",
  },
  eject: { d: "M5 15h14L12 6zM5 19h14" },
};

export function CapGlyph({ icon }: { icon: CapIcon }) {
  const p = PATHS[icon];
  return (
    <svg className="cap-icon" data-icon={icon} viewBox="0 0 24 24" aria-hidden="true">
      <path
        d={p.d}
        fill={p.fill ? "currentColor" : "none"}
        stroke={p.fill ? "none" : "currentColor"}
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export type KeycapProps = { code: string; mac?: boolean; size?: "sm" | "md" | "lg"; lit?: boolean };

/** One key drawn like the 3D keyboard's caps: dark cap, white legend, Windows logo on Win. */
export default function Keycap({ code, mac, size = "md", lit }: KeycapProps) {
  const face = capFace(code, mac);
  const cls = ["cap", `cap-${size}`, face.wide ? "cap-wide" : "", lit ? "cap-lit" : ""].filter(Boolean).join(" ");
  return (
    <span className={cls} data-code={code} aria-hidden="true">
      {face.icon && <CapGlyph icon={face.icon} />}
      {face.legend && <span className="cap-text">{face.legend}</span>}
    </span>
  );
}

/** A chord as caps joined by "+": Win + V. */
export function ChordCaps({ keys, mac, size = "sm" }: { keys: string[]; mac?: boolean; size?: "sm" | "md" }) {
  return (
    <span className="cap-chord">
      {keys.map((k, i) => (
        <span key={`${k}-${i}`} className="cap-chord-part">
          {i > 0 && <span className="cap-plus">+</span>}
          <Keycap code={k} mac={mac} size={size} />
        </span>
      ))}
    </span>
  );
}
