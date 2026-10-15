import { useEffect, useRef, useState } from "react";

/** A plain confirmation toast (e.g. "Fn reset across 3 profiles"), shown bottom-left for a few
 * seconds then faded. Driven by the toast bus through App. */
export default function PlainToast({ message, onDone }: { message: string; onDone: () => void }) {
  const [leaving, setLeaving] = useState(false);
  const done = useRef(onDone);
  done.current = onDone;
  useEffect(() => {
    const out = setTimeout(() => setLeaving(true), 3600);
    const gone = setTimeout(() => done.current(), 4000);
    return () => {
      clearTimeout(out);
      clearTimeout(gone);
    };
  }, []);
  return (
    <div className={"toast toast-plain" + (leaving ? " leaving" : "")} role="status" aria-live="polite">
      <span className="toast-dot" aria-hidden="true" />
      <div className="toast-body">{message}</div>
    </div>
  );
}
