import { useEffect } from "react";
import { onKeyEvent, startRecord, stopRecord } from "./ipc";
import type { KeyEvent } from "./types";

// The engine has one Record mode, so one recording owns it at a time; a new one ends the old.
let owner = 0;
let endOwner: (() => void) | null = null;

/**
 * Starts a recording and returns its cleanup. Keys go to the handler `start` makes until it returns
 * true; `end` runs then, on the engine's 15 s timeout, when Record could not start, or when another
 * recording takes over. Only the current owner's cleanup leaves Record mode.
 */
export function beginRecord(start: () => (e: KeyEvent) => boolean, end: () => void): () => void {
  endOwner?.();
  const me = ++owner;
  endOwner = end;
  const onKey = start();
  let live = true;
  const unlisten = onKeyEvent((e) => {
    if (!live || owner !== me || !onKey(e)) return;
    live = false;
    end();
  });
  unlisten
    .then(() => (live && owner === me ? startRecord() : undefined)) // cleaned up before listen() resolved: never start
    .catch((err) => {
      console.error(err);
      end();
    });
  const timer = setTimeout(end, 15_000); // engine RECORD_TIMEOUT
  return () => {
    live = false;
    clearTimeout(timer);
    unlisten.then((off) => off()).catch(() => undefined);
    if (owner !== me) return; // taken over: the new owner keeps Record on
    endOwner = null;
    stopRecord().catch(console.error); // also puts back Monitor mode if the Keyboard view wants it
  };
}

/** Engine Record mode while `active`; see beginRecord. `start` and `end` are read once per recording. */
export function useRecord(active: boolean, start: () => (e: KeyEvent) => boolean, end: () => void) {
  useEffect(() => (active ? beginRecord(start, end) : undefined), [active]);
}
