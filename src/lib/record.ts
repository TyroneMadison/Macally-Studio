import { useEffect } from "react";
import { onKeyEvent, startRecord, stopRecord } from "./ipc";
import type { KeyEvent } from "./types";

/**
 * Engine Record mode while `active`: keys are swallowed and reported here instead of acting.
 * Each key goes to the handler `start` makes (a fresh one per recording) until it returns true;
 * `end` runs then, on the engine's 15 s timeout, or when Record could not start.
 */
export function useRecord(active: boolean, start: () => (e: KeyEvent) => boolean, end: () => void) {
  useEffect(() => {
    if (!active) return;
    const onKey = start();
    let live = true;
    const unlisten = onKeyEvent((e) => {
      if (!live || !onKey(e)) return;
      live = false;
      end();
    });
    unlisten
      .then(() => (live ? startRecord() : undefined)) // cleaned up before listen() resolved: never start
      .catch((err) => {
        console.error(err);
        end();
      });
    const timer = setTimeout(end, 15_000); // engine RECORD_TIMEOUT
    return () => {
      live = false;
      clearTimeout(timer);
      unlisten.then((off) => off()).catch(() => undefined);
      stopRecord().catch(console.error); // also puts back Monitor mode if the Keyboard view wants it
    };
  }, [active]); // start and end are read once per recording
}
