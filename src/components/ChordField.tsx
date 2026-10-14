import { useEffect, useRef, useState } from "react";
import { formatChord, isMac, parseChord, recordStep, type Mod } from "../lib/actions";
import { onKeyEvent, setMonitor, startRecord, stopRecord } from "../lib/ipc";

export type ChordFieldProps = {
  id: string;
  initial: string[];
  disabled?: boolean;
  onChange: (keys: string[]) => void;
};

/** Typed chord ("Ctrl + Shift + S") plus Record, which captures the next chord through the engine. */
export default function ChordField({ id, initial, disabled = false, onChange }: ChordFieldProps) {
  const [text, setText] = useState(() => formatChord(initial));
  const [recording, setRecording] = useState(false);
  const mounted = useRef(false);

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  // Record mode: the engine swallows keys and reports them as "key-event" until stopRecord.
  useEffect(() => {
    if (!recording) return;
    let held: Mod[] = [];
    let live = true;
    const unlisten = onKeyEvent((e) => {
      if (!live) return;
      const step = recordStep(held, e);
      held = step.held;
      if (!step.done) return;
      live = false;
      setText(formatChord(step.done));
      onChange(step.done);
      setRecording(false);
    });
    unlisten
      .then(() => startRecord())
      .catch((err) => {
        console.error(err);
        setRecording(false);
      });
    const timer = setTimeout(() => setRecording(false), 15_000); // engine RECORD_TIMEOUT
    return () => {
      live = false;
      clearTimeout(timer);
      unlisten.then((off) => off()).catch(() => undefined);
      // Record replaced the Keyboard view's Monitor mode; put it back unless the view is gone.
      stopRecord()
        .then(() => (mounted.current ? setMonitor(true) : undefined))
        .catch(console.error);
    };
  }, [recording]); // onChange is read once per recording

  return (
    <div className="chord">
      <input
        id={id}
        className="input mono"
        value={text}
        placeholder={isMac ? "Cmd + Shift + 4" : "Win + Shift + S"}
        disabled={disabled || recording}
        onChange={(e) => {
          setText(e.target.value);
          onChange(parseChord(e.target.value));
        }}
      />
      <button
        type="button"
        className={"btn" + (recording ? " rec" : "")}
        aria-pressed={recording}
        disabled={disabled}
        onClick={() => setRecording((r) => !r)}
      >
        {recording ? "Press keys…" : "Record"}
      </button>
    </div>
  );
}
