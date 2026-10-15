import { useEffect, useRef, useState } from "react";
import GameTile from "./GameTile";
import { bindingCount } from "../lib/games";
import type { Game } from "../lib/types";

export type GameToastProps = { game: Game; profileName: string; bindings: number; auto: boolean; onDone: () => void };

/** How long a detection notice stays up, and its fade out (matches .toast.leaving in styles.css). */
const SHOW_MS = 4000;
const FADE_MS = 400;

/** "Valorant detected · Switched to Valorant profile": slides in after the corner icon pops, then fades. */
export default function GameToast({ game, profileName, bindings, auto, onDone }: GameToastProps) {
  const [leaving, setLeaving] = useState(false);
  const done = useRef(onDone);
  done.current = onDone;
  useEffect(() => {
    const out = setTimeout(() => setLeaving(true), SHOW_MS);
    const gone = setTimeout(() => done.current(), SHOW_MS + FADE_MS);
    return () => {
      clearTimeout(out);
      clearTimeout(gone);
    };
  }, []);
  return (
    <div className={"toast" + (leaving ? " leaving" : "")} role="status" aria-live="polite">
      <GameTile name={game.name} icon={game.icon} size="lg" />
      <div className="toast-text">
        <div className="toast-title">{game.name} detected</div>
        <div className="toast-body">
          Switched to <b>{profileName}</b> profile · {bindingCount(bindings)}
        </div>
      </div>
      {auto && <span className="gs-pill">AUTO</span>}
    </div>
  );
}
