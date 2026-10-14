import type { KeyEvent } from "../lib/types";

/** Monitor strip: what the OS reported for the last key (engine Monitor mode). */
export default function KeyMonitor({ last }: { last: KeyEvent | null }) {
  return (
    <div className="km" role="status" aria-live="polite" aria-label="Key monitor">
      {last ? (
        <>
          <span>
            last {last.down ? "press" : "release"} · <b>{last.code}</b>
            {last.mods.length > 0 && ` with ${last.mods.join(" + ")}`}
          </span>
          <span>{last.raw}</span>
        </>
      ) : (
        <span>Press any key to see what the OS reports</span>
      )}
    </div>
  );
}
