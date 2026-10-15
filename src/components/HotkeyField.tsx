import { useEffect, useState } from "react";

type Props = {
  id: string;
  value: string;
  onChange: (value: string) => void;
};

/** Typed shortcut ("Ctrl+Alt+X"), saved on Enter or when the field loses focus. */
export default function HotkeyField({ id, value, onChange }: Props) {
  const [text, setText] = useState(value);
  useEffect(() => setText(value), [value]);
  const commit = () => {
    if (text.trim() !== value) onChange(text.trim());
  };
  return (
    <input
      id={id}
      className="input mono hotkey-field"
      value={text}
      spellCheck={false}
      placeholder="Ctrl+Alt+X"
      onChange={(e) => setText(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => e.key === "Enter" && commit()}
    />
  );
}
