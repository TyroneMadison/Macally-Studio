import { useState, type ReactNode } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import HotkeyField from "../components/HotkeyField";
import { checkUpdate, installUpdate } from "../lib/ipc";
import type { AppState } from "../lib/state";
import type { Hotkeys, Settings as SettingsData, SwitchMode, UpdateInfo } from "../lib/types";
import { displayVersion } from "../lib/version";

const IS_WINDOWS = navigator.userAgent.includes("Windows");
const REPO = "https://github.com/TyroneMadison/Macally-Studio";

type ToggleKey = "startAtLogin" | "closeToTray" | "runAsAdmin" | "windowsLegends";
const TOGGLES: { key: ToggleKey; title: string; hint: string }[] = [
  { key: "startAtLogin", title: "Start at login", hint: "Launch hidden in the tray so remaps are always on" },
  { key: "closeToTray", title: "Close window to tray", hint: "Closing keeps remapping running. Quit from the tray menu." },
  { key: "runAsAdmin", title: "Run as administrator", hint: "Needed for remaps to work inside admin-elevated windows" },
  { key: "windowsLegends", title: "Show Windows legends on the 3D keyboard", hint: "Off shows the keycaps exactly as printed (Mac legends)" },
];

const MODES: { id: SwitchMode; title: string; hint: string }[] = [
  { id: "manual", title: "Manual", hint: "Pick from the app or tray" },
  { id: "hotkey", title: "Hotkey", hint: "Cycle with a shortcut" },
  { id: "auto", title: "Auto per-app", hint: "Follows the app in front" },
];

const HOTKEYS: { key: keyof Hotkeys; label: string }[] = [
  { key: "pause", label: "Pause / resume remapping" },
  { key: "nextProfile", label: "Next profile" },
  { key: "prevProfile", label: "Previous profile" },
  { key: "show", label: "Show Macally Studio" },
];

function Card({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="card">
      <div className="kb-label card-title">{title}</div>
      {children}
    </section>
  );
}

function ExtLink({ href, children }: { href: string; children: ReactNode }) {
  return (
    <a
      href={href}
      onClick={(e) => {
        e.preventDefault();
        void openUrl(href);
      }}
    >
      {children}
    </a>
  );
}

function StatusRow({ ok, label, value }: { ok: boolean; label: string; value: string }) {
  return (
    <div className="setting-row">
      <span className={"dot" + (ok ? " ok" : "")} />
      <span className="grow strong">{label}</span>
      <span className="muted small">{value}</span>
    </div>
  );
}

function UpdatesCard({ store }: { store: AppState }) {
  const [found, setFound] = useState<UpdateInfo | null>(null);
  const [checkedAt, setCheckedAt] = useState<Date | null>(null);
  const [busy, setBusy] = useState<"check" | "install" | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const update = found ?? store.update;

  const check = async () => {
    setBusy("check");
    setNote(null);
    try {
      const result = await checkUpdate();
      setFound(result);
      setCheckedAt(new Date());
      if (!result) setNote("You have the latest version.");
    } catch (e) {
      setNote(`Could not check for updates: ${e}`);
    } finally {
      setBusy(null);
    }
  };
  const install = async () => {
    setBusy("install");
    try {
      await installUpdate(); // the app restarts when this succeeds
    } catch (e) {
      setNote(`Could not install the update: ${e}`);
      setBusy(null);
    }
  };

  return (
    <Card title="Updates">
      <div className="card-pad">
        <div className="row gap-16">
          <div className="app-mark">MS</div>
          <div className="col gap-3">
            <div className="big">Macally Studio</div>
            <div className="mono">Version {store.status ? displayVersion(store.status.version) : "-"}</div>
            <div className="muted small">
              {checkedAt
                ? `Last checked today, ${checkedAt.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" })}`
                : "Not checked since the app started"}
            </div>
          </div>
        </div>
        {update && (
          <div className="banner">
            <span className="dot primary" />
            <div className="col grow gap-3">
              <div className="strong">Version {displayVersion(update.version)} is available</div>
              <div className="small banner-notes">
                {update.notes}{" "}
                <ExtLink href={`${REPO}/releases/tag/v${displayVersion(update.version)}`}>Release notes</ExtLink>
              </div>
            </div>
            <button type="button" className="btn primary" disabled={busy !== null} onClick={() => void install()}>
              {busy === "install" ? "Installing…" : "Install and relaunch"}
            </button>
          </div>
        )}
        <div className="row gap-10 wrap">
          <button type="button" className="btn" disabled={busy !== null} onClick={() => void check()}>
            {busy === "check" ? "Checking…" : "Check for updates"}
          </button>
          <label className="check">
            <input
              type="checkbox"
              checked={store.settings.autoCheckUpdates}
              onChange={(e) => void store.saveSettings({ autoCheckUpdates: e.target.checked })}
            />
            Check automatically on launch
          </label>
        </div>
        {note && <div className="small muted">{note}</div>}
        <div className="muted small">
          Versions go 1.0, 1.1 … 1.9, then 2.0. Updates come straight from the GitHub Releases page.
        </div>
      </div>
    </Card>
  );
}

export default function Settings({ store }: { store: AppState }) {
  const s = store.settings;
  const st = store.status;
  const save = (patch: Partial<SettingsData>) => void store.saveSettings(patch);

  return (
    <div className="page">
      <header className="page-head">
        <h1>Settings</h1>
      </header>
      <section className="page-body">
        <div className="settings-col">
          <Card title="General">
            {TOGGLES.filter((t) => IS_WINDOWS || t.key !== "runAsAdmin").map((t) => (
              <label key={t.key} className="setting-row clickable">
                <span className="col grow gap-3">
                  <span className="strong">{t.title}</span>
                  <span className="muted small">{t.hint}</span>
                </span>
                <input
                  type="checkbox"
                  role="switch"
                  className="switch"
                  checked={s[t.key]}
                  onChange={(e) => save({ [t.key]: e.target.checked })}
                />
              </label>
            ))}
          </Card>

          <Card title="Profile switching">
            <div className="card-pad">
              <div role="group" aria-label="Switch mode" className="modes">
                {MODES.map((m) => (
                  <button
                    key={m.id}
                    type="button"
                    aria-pressed={s.switchMode === m.id}
                    onClick={() => save({ switchMode: m.id })}
                  >
                    <span className="strong">{m.title}</span>
                    <span className="muted small">{m.hint}</span>
                  </button>
                ))}
              </div>
              <div className="muted small">
                In Auto mode, apps without a rule fall back to the profile you last picked manually.
              </div>
            </div>
          </Card>

          <Card title="Hotkeys">
            {HOTKEYS.map((h) => (
              <div key={h.key} className="setting-row">
                <label htmlFor={`hotkey-${h.key}`} className="grow strong">
                  {h.label}
                </label>
                <HotkeyField
                  id={`hotkey-${h.key}`}
                  value={s.hotkeys[h.key]}
                  onChange={(v) => save({ hotkeys: { ...s.hotkeys, [h.key]: v } })}
                />
              </div>
            ))}
          </Card>
        </div>

        <div className="settings-col side">
          <UpdatesCard store={store} />

          <Card title="Permissions and device">
            <StatusRow ok={!!st?.hookRunning} label="Keyboard hook" value={st?.hookRunning ? "Running" : "Not running"} />
            <StatusRow
              ok={!!st?.deviceFound}
              label="Macally keyboard"
              value={st?.deviceFound ? `USB · VID 2222 · PID 0114 · ${store.layout.keys.length} keys` : "Not detected"}
            />
            <StatusRow
              ok={!IS_WINDOWS && !!st?.permissionsOk}
              label="Accessibility and Input Monitoring"
              value={IS_WINDOWS ? "macOS only" : st?.permissionsOk ? "Granted" : "Needed"}
            />
          </Card>

          <section className="card card-pad">
            <div className="kb-label">About</div>
            <p className="about">
              Open-source remapping studio for the Macally backlit Mac keyboard on Windows and macOS. Made by Tyrone
              Madison.
            </p>
            <div className="row gap-14 small strong">
              <ExtLink href={REPO}>GitHub</ExtLink>
              <ExtLink href={`${REPO}/issues`}>Report a problem</ExtLink>
              <ExtLink href={`${REPO}/blob/main/LICENSE`}>License</ExtLink>
            </div>
          </section>
        </div>
      </section>
    </div>
  );
}
