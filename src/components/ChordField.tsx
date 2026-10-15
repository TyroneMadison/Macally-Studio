import { useState } from "react";
import { formatChord, isMac, parseChord } from "../lib/actions";

export type ChordFieldProps = {
  id: string;
  initial: string[];
  disabled?: boolean;
  onChange: (keys: string[]) => void;
};

/** Typed chord: "Ctrl + Shift + S". */
export default function ChordField({ id, initial, disabled = false, onChange }: ChordFieldProps) {
  const [text, setText] = useState(() => formatChord(initial));
  return (
    <input
      id={id}
      className="input mono"
      value={text}
      placeholder={isMac ? "Cmd + Shift + 4" : "Win + Shift + S"}
      disabled={disabled}
      onChange={(e) => {
        setText(e.target.value);
        onChange(parseChord(e.target.value));
      }}
    />
  );
}
