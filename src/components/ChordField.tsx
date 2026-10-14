import { useState } from "react";
import { chordRecorder, formatChord, isMac, parseChord } from "../lib/actions";
import { useRecord } from "../lib/record";

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
  useRecord(
    recording,
    () =>
      chordRecorder((keys) => {
        setText(formatChord(keys));
        onChange(keys);
      }),
    () => setRecording(false),
  );

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
