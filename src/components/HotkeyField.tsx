import { useEffect, useRef, useState } from "react";
import { formatHotkey } from "../lib/hotkey";
import { onKeyEvent, startRecord, stopRecord } from "../lib/ipc";

type Props = {
  id: string;
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
};

/** Shortcut text field ("Ctrl+Alt+X") with a Record button that captures the next real key press. */
export default function HotkeyField({ id, value, onChange, disabled }: Props) {
  const [text, setText] = useState(value);
  const [recording, setRecording] = useState(false);
  const latest = useRef(onChange);

  useEffect(() => setText(value), [value]);
  useEffect(() => {
    latest.current = onChange;
  });

  useEffect(() => {
    if (!recording) return;
    let done = false;
    const unlisten = onKeyEvent((ev) => {
      const combo = formatHotkey(ev);
      if (!combo || done) return;
      done = true;
      setRecording(false);
      latest.current(combo);
    });
    void startRecord();
    const timer = setTimeout(() => setRecording(false), 15_000); // engine RECORD_TIMEOUT
    return () => {
      clearTimeout(timer);
      void stopRecord();
      void unlisten.then((un) => un());
    };
  }, [recording]);

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
        disabled={disabled}
        spellCheck={false}
        onChange={(e) => setText(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => e.key === "Enter" && commit()}
      />
      <button
        type="button"
        className={"btn" + (recording ? " primary" : "")}
        disabled={disabled}
        onClick={() => setRecording((r) => !r)}
      >
        {recording ? "Cancel" : "Record"}
      </button>
    </div>
  );
}
