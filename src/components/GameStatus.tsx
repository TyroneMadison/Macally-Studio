import GameTile from "./GameTile";
import type { Game } from "../lib/types";

export type GameStatusProps = {
  /** The active profile's game; null while no game profile is active. */
  game: Game | null;
  /** Game detection is on (Settings.gameAutoSwitch). */
  watching: boolean;
  /** The game was detected in front (its profile switched in by itself). */
  live: boolean;
  /** Its profile switches automatically. */
  auto: boolean;
  /** Changes on each detection, so the icon pops in again. */
  pop?: number;
  onOpen: () => void;
};

const PAD = "M7 8h10a5 5 0 0 1 0 10c-1.6 0-2.4-.8-3.2-2h-3.6c-.8 1.2-1.6 2-3.2 2a5 5 0 0 1 0-10zM8 11v4M6 13h4M15.5 12h.01M17.5 14h.01";

/** Bottom-left of the rail, where play/pause was: the game in front, or watching for one. */
export default function GameStatus({ game, watching, live, auto, pop, onOpen }: GameStatusProps) {
  if (!game) {
    const caption = watching ? "Watching for games" : "Game detection off";
    return (
      <button type="button" className="gs gs-idle" aria-label={`${caption}. Open game profiles`} onClick={onOpen}>
        <span className={"gs-watch" + (watching ? "" : " off")}>
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
            <path d={PAD} />
          </svg>
        </span>
        <span className="gs-caption">{caption}</span>
      </button>
    );
  }
  const what = live ? `${game.name} is in front, its profile is active` : `${game.name}'s profile is active`;
  return (
    <button type="button" className="gs" aria-label={`${what}. Open game profiles`} onClick={onOpen}>
      <span key={pop} className={"gs-icon" + (pop ? " gs-pop" : "")}>
        <GameTile name={game.name} icon={game.icon} size="md" />
        {live && <span className="gs-live" />}
      </span>
      <span className="gs-name">{game.name}</span>
      <span className={"gs-pill" + (auto ? "" : " manual")}>{auto ? "AUTO" : "MANUAL"}</span>
    </button>
  );
}
