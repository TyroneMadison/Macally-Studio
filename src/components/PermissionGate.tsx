import { useEffect, useState, type ReactNode } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getStatus } from "../lib/ipc";
import type { AppState } from "../lib/state";
import type { Status } from "../lib/types";

const PRIVACY = "x-apple.systempreferences:com.apple.preference.security?Privacy_";

const PERMISSIONS = [
  {
    name: "Accessibility",
    pane: "Accessibility",
    why: "Lets Macally Studio send the keys and shortcuts your profile maps to.",
  },
  {
    name: "Input Monitoring",
    pane: "ListenEvent",
    why: "Lets Macally Studio see which key you pressed.",
  },
];

/** True only on macOS while Accessibility or Input Monitoring is still off. */
export function needsPermissions(status: Status | null): boolean {
  return status?.platform === "macos" && !status.permissionsOk;
}

/**
 * macOS: covers the whole window until both permissions are on, checking every 2 s.
 * The engine retries its event tap on its own, so remapping starts as soon as they are granted.
 */
export default function PermissionGate({ store, children }: { store: AppState; children: ReactNode }) {
  const { refresh } = store;
  const [status, setStatus] = useState(store.status);
  useEffect(() => setStatus(store.status), [store.status]);
  const blocked = needsPermissions(status);

  useEffect(() => {
    if (!blocked) return;
    const timer = setInterval(() => {
      getStatus()
        .then((s) => {
          setStatus(s);
          if (!needsPermissions(s)) void refresh();
        })
        .catch(() => {});
    }, 2000);
    return () => clearInterval(timer);
  }, [blocked, refresh]);

  if (!blocked) return <>{children}</>;
  return (
    <main className="gate">
      <section className="gate-card" aria-labelledby="gate-title">
        <div className="app-mark">MS</div>
        <h1 id="gate-title">Macally Studio needs two permissions</h1>
        <p className="muted">
          macOS only lets an app read and remap your keyboard after you turn these on in System Settings, under
          Privacy &amp; Security.
        </p>
        {PERMISSIONS.map((p) => (
          <div key={p.pane} className="gate-row">
            <div className="col grow gap-3">
              <span className="strong">{p.name}</span>
              <span className="muted small">{p.why}</span>
            </div>
            <button type="button" className="btn primary" onClick={() => void openUrl(PRIVACY + p.pane)}>
              Open {p.name} settings
            </button>
          </div>
        ))}
        <div className="row gap-10 small muted">
          <span className="dot primary" />
          Waiting for both switches. This screen closes by itself.
        </div>
        <p className="muted small">
          Still here after both are on? Quit Macally Studio from the menu bar and open it again.
        </p>
      </section>
    </main>
  );
}
