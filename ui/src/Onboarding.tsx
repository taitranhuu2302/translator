import { useState } from "react";

import { formatShortcut } from "./format";
import type { ShortcutBindings } from "./ipc";

const SHORTCUTS: { key: keyof ShortcutBindings; name: string; action: string }[] = [
  {
    key: "quick_translate",
    name: "Quick Translate",
    action: "Translate the selection in a popup beside your cursor.",
  },
  {
    key: "replace",
    name: "Replace",
    action: "Translate the selection and overwrite it in place.",
  },
  {
    key: "read_aloud",
    name: "Read Aloud",
    action: "Hear the selection in the system voice.",
  },
  {
    key: "translate_window",
    name: "Translate Window",
    action: "Open this window with the selection already in it.",
  },
];

export function Onboarding({
  shortcuts,
  onDone,
}: {
  shortcuts: ShortcutBindings;
  onDone: () => Promise<void>;
}) {
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const finish = async () => {
    setSaving(true);
    setError(null);
    try {
      await onDone();
    } catch {
      setError("The onboarding could not be saved. Try again.");
      setSaving(false);
    }
  };

  return (
    <div className="onboarding-backdrop">
      <section
        className="onboarding"
        role="dialog"
        aria-modal="true"
        aria-labelledby="onboarding-title"
      >
        <h2 id="onboarding-title">Welcome to Translator</h2>
        <p className="muted">
          Translator lives in the menu bar and works on whatever you select in
          another app. Select text anywhere, then use one of these.
        </p>

        <ul className="shortcut-list">
          {SHORTCUTS.map((shortcut) => (
            <li className="shortcut-row" key={shortcut.key}>
              <kbd>{formatShortcut(shortcuts[shortcut.key])}</kbd>
              <div>
                <strong>{shortcut.name}</strong>
                <span className="muted">{shortcut.action}</span>
              </div>
            </li>
          ))}
        </ul>

        <p className="muted small">
          Reading your selection needs the Accessibility permission. macOS asks
          the first time a shortcut fires; you can also grant it now from
          System Settings → Privacy &amp; Security → Accessibility.
        </p>

        {error ? <p className="error">{error}</p> : null}

        <button className="primary" onClick={finish} disabled={saving}>
          {saving ? "Saving…" : "Got it"}
        </button>
      </section>
    </div>
  );
}
