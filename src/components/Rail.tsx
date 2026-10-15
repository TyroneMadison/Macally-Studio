import type { ReactNode } from "react";

export type View = "keyboard" | "profiles" | "settings" | "games";

export type RailProps = {
  view: View;
  onNav: (v: View) => void;
  /** The game status, at the bottom. */
  corner?: ReactNode;
};

// 24x24 stroke icon paths
const ICONS: Record<View, string> = {
  keyboard: "M3 7h18v10H3zM7 10h.01M11 10h.01M15 10h.01M7 14h10",
  profiles: "M12 3l9 5-9 5-9-5 9-5zM3 13l9 5 9-5",
  settings: "M12 8a4 4 0 100 8 4 4 0 000-8zM4 12h-1M21 12h-1M12 4V3M12 21v-1M6.3 6.3l-.7-.7M18.4 18.4l-.7-.7M6.3 17.7l-.7.7M18.4 5.6l-.7.7",
  games: "M7 8h10a5 5 0 0 1 0 10c-1.6 0-2.4-.8-3.2-2h-3.6c-.8 1.2-1.6 2-3.2 2a5 5 0 0 1 0-10zM8 11v4M6 13h4M15.5 12h.01M17.5 14h.01",
};

const NAV: { id: View; label: string }[] = [
  { id: "keyboard", label: "Keyboard" },
  { id: "profiles", label: "Profiles" },
  { id: "settings", label: "Settings" },
  { id: "games", label: "Game profiles" },
];

function Icon({ d }: { d: string }) {
  return (
    <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d={d} />
    </svg>
  );
}

export default function Rail({ view, onNav, corner }: RailProps) {
  return (
    <nav className="rail" aria-label="Main">
      <div className="rail-mark" title="Macally Studio">M</div>
      {NAV.map((n) => (
        <button
          key={n.id}
          type="button"
          className={"rail-btn" + (view === n.id ? " active" : "")}
          title={n.label}
          aria-label={n.label}
          aria-current={view === n.id ? "page" : undefined}
          onClick={() => onNav(n.id)}
        >
          <Icon d={ICONS[n.id]} />
        </button>
      ))}
      <div className="rail-spacer" />
      {corner}
    </nav>
  );
}
