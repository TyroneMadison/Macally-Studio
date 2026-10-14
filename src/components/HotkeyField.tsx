import { useEffect, useState } from "react";
import { formatHotkey } from "../lib/hotkey";
import { useRecord } from "../lib/record";

type Props = {
  id: string;
  value: string;
  onChange: (value: string) => void;
};

/** Shortcut text field ("Ctrl+Alt+X") with a Record button that captures the next real key press. */
export default function HotkeyField({ id, value, onChange }: Props) {
  const [text, setText] = useState(value);
  const [recording, setRecording] = useState(false);

  useEffect(() => setText(value), [value]);
  useRecord(
    recording,
    () => (ev) => {
      const combo = formatHotkey(ev);
      if (combo) onChange(combo);
      return combo !== null;
    },
    () => setRecording(false),
  );

  const commit = () => {
    if (text.trim() !== value) onChange(text.trim());
  };

  return (
    <div className="hotkey-field">
      <input
        id={id}
        className="input mono"
        value={recording ? "Press a shortcut…" : text}
        readOnly={recording}
        spellCheck={false}
        onChange={(e) => setText(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => e.key === "Enter" && commit()}
      />
      <button
        type="button"
        className={"btn" + (recording ? " primary" : "")}
        onClick={() => setRecording((r) => !r)}
      >
        {recording ? "Cancel" : "Record"}
      </button>
    </div>
  );
}
