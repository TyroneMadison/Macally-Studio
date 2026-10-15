import { convertFileSrc } from "@tauri-apps/api/core";
import { tileColors } from "../lib/games";

export type TileSize = "sm" | "md" | "lg" | "xl";

/** A game's icon, or its first letter on a colour of its own, like the mockups' tiles. */
export default function GameTile({ name, icon, size = "md" }: { name: string; icon?: string | null; size?: TileSize }) {
  if (icon) return <img className={`tile tile-${size}`} src={convertFileSrc(icon)} alt="" />;
  const [from, to] = tileColors(name);
  return (
    <span className={`tile tile-${size} tile-letter`} style={{ background: `linear-gradient(160deg, ${from}, ${to})` }} aria-hidden="true">
      {(name.trim()[0] ?? "?").toUpperCase()}
    </span>
  );
}
