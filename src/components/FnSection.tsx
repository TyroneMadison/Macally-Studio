import { useState } from "react";
import { isMac } from "../lib/actions";
import type { AppState } from "../lib/state";
import { showToast } from "../lib/toast";
import type { Binding, Profile } from "../lib/types";

export type FnSectionProps = {
  /** The profile being edited (a game profile, or the Default). */
  profile: Profile;
  saveBinding: AppState["saveBinding"];
  resetKeyEverywhere: AppState["resetKeyEverywhere"];
  /** Opens another key in the inspector, where it can be remapped to anything. */
  onSelectKey?: (code: string) => void;
};

/** The keys next to fn, as printed on the Macally, with what they are on Windows. */
const NEARBY: { code: string; label: string }[] = [
  { code: "ControlLeft", label: isMac ? "Left Control" : "Left Control (Ctrl)" },
  { code: "CapsLock", label: "Caps Lock" },
  { code: "MetaLeft", label: isMac ? "Left Command" : "Left Command (Win)" },
];

const capsIsCtrl = (p: Profile) => {
  const a = p.bindings.CapsLock?.action;
  return a?.type === "key_combo" && a.keys.length === 1 && (a.keys[0] === "ControlLeft" || a.keys[0] === "Ctrl");
};

/** Shown in the inspector when Fn is selected: Fn is firmware-only and cannot be remapped. Offers
 * the realistic workaround (Caps Lock as Ctrl) and a one-click reset that clears any Fn remap left
 * over from before, across every profile. */
export default function FnSection({ profile, saveBinding, resetKeyEverywhere, onSelectKey }: FnSectionProps) {
  const [busy, setBusy] = useState(false);
  const capsCtrl = capsIsCtrl(profile);

  const useCapsAsCtrl = () => {
    const binding: Binding = { mods: [], action: { type: "key_combo", keys: ["ControlLeft"] } };
    void saveBinding(profile.id, "CapsLock", () => binding).then(() => showToast("Caps Lock now sends Ctrl"));
  };
  const resetFn = () => {
    setBusy(true);
    resetKeyEverywhere("Fn")
      .then((n) => showToast(n > 0 ? `Fn reset across ${n} profile${n === 1 ? "" : "s"}` : "Fn was already clear"))
      .catch((e) => showToast(`Could not reset Fn: ${e}`))
      .finally(() => setBusy(false));
  };

  return (
    <div className="fnsec">
      <div className="insp-card fnsec-why">
        <div className="kb-label">About the Fn key</div>
        <p className="strong">Fn can’t be remapped</p>
        <p className="muted">
          Fn is handled inside the keyboard’s own firmware and never reaches the computer, so no app — this one included
          — can see it or change what it does. The keyboard sends nothing to Windows when you press Fn.
        </p>
        <p className="muted">
          What fn still does: <b>fn + esc</b> turns Fn-Lock on or off. With Fn-Lock on, the top row sends F1–F15, and
          those keys can be remapped here; with it off, they send the printed media shortcuts.
        </p>
      </div>

      {onSelectKey && (
        <div className="insp-field">
          <div className="kb-label">Remap a nearby key instead</div>
          <p className="muted">Pick one to give it any action in this profile: a key, a shortcut or a program.</p>
          <div className="fnsec-keys">
            {NEARBY.map((k) => (
              <button key={k.code} type="button" className="btn" data-code={k.code} onClick={() => onSelectKey(k.code)}>
                {k.label}
              </button>
            ))}
          </div>
        </div>
      )}

      <div className="insp-field">
        <div className="kb-label">Reaching for Ctrl?</div>
        {capsCtrl ? (
          <p className="muted">Caps Lock already sends Ctrl in this profile — use it as your bottom-left Ctrl.</p>
        ) : (
          <>
            <p className="muted">
              In games many people hit the bottom-left key for Ctrl. Since Fn can’t act as Ctrl, map Caps Lock to Ctrl
              and use that instead.
            </p>
            <button type="button" className="btn primary" onClick={useCapsAsCtrl}>
              Use Caps Lock as Ctrl
            </button>
          </>
        )}
      </div>

      <div className="insp-spacer" />
      <div className="insp-field">
        <div className="kb-label">Stuck remap?</div>
        <p className="muted">
          If you mapped Fn before, that binding did nothing. Clear it from every profile, including game profiles.
        </p>
        <button type="button" className="btn" disabled={busy} onClick={resetFn}>
          Reset Fn key
        </button>
      </div>
    </div>
  );
}
