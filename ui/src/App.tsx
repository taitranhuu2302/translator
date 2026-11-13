import { useCallback, useEffect, useState } from "react";

import { formatShortcut, captureFailure, type CaptureFailure } from "./format";
import {
  ipc,
  onSelectionPrefill,
  type Prefill,
  type Settings,
} from "./ipc";
import { Onboarding } from "./Onboarding";

const FALLBACK_SHORTCUT = "Ctrl+Alt+E";

export function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [settingsError, setSettingsError] = useState<string | null>(null);

  /// null until the shell answers, so a missing grant is never assumed.
  const [granted, setGranted] = useState<boolean | null>(null);

  const [text, setText] = useState("");
  const [failure, setFailure] = useState<CaptureFailure | null>(null);

  const applyPrefill = useCallback((prefill: Prefill | null) => {
    if (!prefill) return;
    if ("selection" in prefill) {
      setText(prefill.selection.text);
      setFailure(null);
      return;
    }
    setFailure(captureFailure(prefill.failure));
  }, []);

  // Settings decide whether the onboarding shows and how the shortcut reads.
  useEffect(() => {
    let cancelled = false;
    ipc
      .getSettings()
      .then((loaded) => {
        if (!cancelled) setSettings(loaded);
      })
      .catch((reason: unknown) => {
        if (!cancelled) setSettingsError(JSON.stringify(reason));
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // A grant can happen while the window is open. Re-ask whenever this window
  // comes back to the front, which is what returning from System Settings does.
  useEffect(() => {
    let cancelled = false;
    const check = () => {
      ipc
        .accessibilityGranted()
        .then((isGranted) => {
          if (!cancelled) setGranted(isGranted);
        })
        .catch(() => {
          // Leave the state unknown; the notice then offers both paths.
        });
    };
    check();
    window.addEventListener("focus", check);
    return () => {
      cancelled = true;
      window.removeEventListener("focus", check);
    };
  }, []);

  // The Selection was captured before this window was shown; claim it now and
  // whenever a later shortcut opens the window again.
  useEffect(() => {
    let cancelled = false;
    const claim = () => {
      ipc
        .takeSelectionPrefill()
        .then((prefill) => {
          if (!cancelled) applyPrefill(prefill);
        })
        .catch(() => {
          // No prefill is the same as nothing selected.
        });
    };
    const unlisten = onSelectionPrefill(claim);
    claim();
    return () => {
      cancelled = true;
      void unlisten.then((stop) => stop()).catch(() => {
        // The listener never attached, so there is nothing to stop.
      });
    };
  }, [applyPrefill]);

  const translateShortcut = formatShortcut(
    settings?.shortcuts.translate_window ?? FALLBACK_SHORTCUT,
  );

  const requestAccess = async () => {
    try {
      setGranted(await ipc.requestAccessibility());
    } catch {
      setGranted(false);
    }
  };

  const openAccessSettings = () => {
    void ipc.openAccessibilitySettings().catch(() => {
      // Nothing to say: the System Settings pane may simply not open.
    });
  };

  const finishOnboarding = async () => {
    setSettings(await ipc.completeOnboarding());
  };

  return (
    <main className="app">
      <header className="topbar">
        <div>
          <h1>Translator</h1>
          <p className="muted">
            Acts on the selection in whatever app has focus.
          </p>
        </div>
        <p className="hint">
          <kbd>{translateShortcut}</kbd> opens this window with your selection
        </p>
      </header>

      {settingsError ? (
        <p className="notice error">Settings could not be loaded: {settingsError}</p>
      ) : null}

      {failure ? (
        <CaptureNotice
          failure={failure}
          granted={granted}
          shortcut={translateShortcut}
          onRequestAccess={requestAccess}
          onOpenSettings={openAccessSettings}
        />
      ) : null}

      <div className="panes">
        <section className="pane">
          <h2>Source</h2>
          <textarea
            aria-label="Text to translate"
            className="source-input"
            value={text}
            onChange={(event) => setText(event.target.value)}
            placeholder={`Select text in any app and press ${translateShortcut}, or type here.`}
            spellCheck={false}
            autoFocus
          />
          <p className="muted small">
            {text.length === 0
              ? "Nothing selected — the window opens empty."
              : `${text.length} characters`}
          </p>
        </section>

        <section className="pane">
          <h2>Translation</h2>
          <p className="empty">The translation will appear here.</p>
        </section>
      </div>

      {settings && !settings.onboarding_seen ? (
        <Onboarding
          shortcuts={settings.shortcuts}
          onDone={finishOnboarding}
        />
      ) : null}
    </main>
  );
}

function CaptureNotice({
  failure,
  granted,
  shortcut,
  onRequestAccess,
  onOpenSettings,
}: {
  failure: CaptureFailure;
  granted: boolean | null;
  shortcut: string;
  onRequestAccess: () => void;
  onOpenSettings: () => void;
}) {
  if (failure.kind === "no_accessibility") {
    if (granted) {
      return (
        <p className="notice ok">
          Accessibility is on. Select text in any app and press {shortcut} again.
        </p>
      );
    }
    return (
      <div className="notice attention">
        <div>
          <strong>Translator needs Accessibility to read your selection.</strong>
          <span className="muted">
            macOS treats the selected text as another app's data. Grant it once
            and the shortcut works in every app, including after updates.
          </span>
        </div>
        <div className="actions">
          <button className="primary" onClick={onRequestAccess}>
            Grant Accessibility
          </button>
          <button onClick={onOpenSettings}>Open System Settings</button>
        </div>
      </div>
    );
  }

  if (failure.kind === "secure_input") {
    return (
      <div className="notice warning">
        <div>
          <strong>This is a secure field, so nothing was read.</strong>
          <span className="muted">
            Passwords and other secure input never leave your Mac. Finish in
            that field, then try again somewhere else.
          </span>
        </div>
      </div>
    );
  }

  return (
    <div className="notice">
      <div>
        <strong>The selection could not be read.</strong>
        <span className="muted">{failure.reason}</span>
      </div>
    </div>
  );
}
