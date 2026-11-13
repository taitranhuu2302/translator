import { useCallback, useEffect, useRef, useState } from "react";

import { providerLabel } from "./failures";
import { formatShortcut, captureFailure, type CaptureFailure } from "./format";
import {
  EMPTY_IMPROVEMENT,
  partLabel,
  reduceImprovement,
  type ImproveResult,
} from "./improveWindow";
import {
  ipc,
  onSelectionPrefill,
  onTranslationChunk,
  onTranslationDone,
  onTranslationError,
  type Direction,
  type EngineError,
  type ImprovePart,
  type Mode,
  type Prefill,
  type Settings,
} from "./ipc";
import { Onboarding } from "./Onboarding";
import { ProviderSettings } from "./ProviderSettings";
import {
  EMPTY_RESULT,
  reduceResult,
  swapDirection,
  type TranslateResult,
} from "./translateWindow";

const FALLBACK_SHORTCUT = "Ctrl+Alt+E";

const IMPROVE_PARTS: ImprovePart[] = ["corrected", "better"];

export function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [settingsError, setSettingsError] = useState<string | null>(null);

  /// null until the shell answers, so a missing grant is never assumed.
  const [granted, setGranted] = useState<boolean | null>(null);

  const [text, setText] = useState("");
  const [failure, setFailure] = useState<CaptureFailure | null>(null);
  const [mode, setMode] = useState<Mode>("translate");

  // The result pane, the direction this window translates in (null means the
  // saved Language Pair), and whether the Copy button has just been used.
  const [result, setResult] = useState<TranslateResult>(EMPTY_RESULT);
  const [direction, setDirection] = useState<Direction | null>(null);
  const [copied, setCopied] = useState(false);

  // Improve keeps its own pane: two labelled parts that fill in as the model
  // writes. `copiedPart` says which section's Copy was last used.
  const [improvement, setImprovement] = useState<ImproveResult>(
    EMPTY_IMPROVEMENT,
  );
  const [copiedPart, setCopiedPart] = useState<ImprovePart | null>(null);

  // Every request gets an id; an answer whose id is no longer current belongs to
  // an abandoned translate (the input was cleared, the window reopened, or a
  // newer request started) and must not touch the pane.
  const requestId = useRef(0);

  const applyPrefill = useCallback((prefill: Prefill | null) => {
    if (!prefill) return;
    if ("selection" in prefill) {
      requestId.current += 1;
      setText(prefill.selection.text);
      setFailure(null);
      setResult((state) => reduceResult(state, { type: "cleared" }));
      setImprovement((state) => reduceImprovement(state, { type: "cleared" }));
      // A new opening starts in Translate, from the saved pair again; Swap never
      // survives it.
      setMode("translate");
      setDirection(null);
      setCopied(false);
      setCopiedPart(null);
      return;
    }
    setFailure(captureFailure(prefill.failure));
  }, []);

  const runTranslation = useCallback(
    async (source: string, requested: Direction | null) => {
      const id = ++requestId.current;
      setCopied(false);
      if (source.trim().length === 0) {
        setResult((state) => reduceResult(state, { type: "cleared" }));
        return;
      }
      setResult((state) => reduceResult(state, { type: "started" }));
      try {
        const outcome = await ipc.translateText(source, "translate", requested);
        if (requestId.current !== id) return;
        if ("translation" in outcome) {
          const translation = outcome.translation;
          setResult((state) =>
            reduceResult(state, { type: "translated", translation }),
          );
        }
      } catch (error) {
        if (requestId.current !== id) return;
        setResult((state) =>
          reduceResult(state, { type: "failed", error: error as EngineError }),
        );
      }
    },
    [],
  );

  // Improve streams through events rather than an answer to this promise: the
  // command returns once its worker starts. `done` is the only value the
  // provider strictly validated, so it replaces what the chunks showed.
  const runImprovement = useCallback(async (source: string) => {
    const id = ++requestId.current;
    setCopiedPart(null);
    if (source.trim().length === 0) {
      setImprovement((state) => reduceImprovement(state, { type: "cleared" }));
      return;
    }
    setImprovement((state) =>
      reduceImprovement(state, { type: "started", requestId: id }),
    );
    try {
      await ipc.improveText(String(id), source);
    } catch (error) {
      // The source text is App's `text` state and is never touched here: a
      // failed Improve must leave what the user typed exactly as it was.
      if (requestId.current !== id) return;
      setImprovement((state) =>
        reduceImprovement(state, {
          type: "failed",
          requestId: id,
          error: error as EngineError,
        }),
      );
    }
  }, []);

  const translate = useCallback(() => {
    if (mode === "improve") {
      void runImprovement(text);
      return;
    }
    void runTranslation(text, direction);
  }, [direction, mode, runImprovement, runTranslation, text]);

  const changeText = (value: string) => {
    setText(value);
    setCopied(false);
    setCopiedPart(null);
    // Emptying the input clears both panes and abandons any answer still in
    // flight, so neither pane can show a result for text that is gone.
    if (value.trim().length === 0) {
      requestId.current += 1;
      setResult((state) => reduceResult(state, { type: "cleared" }));
      setImprovement((state) => reduceImprovement(state, { type: "cleared" }));
    }
  };

  // Switching mode keeps an in-flight stream alive in its own pane: the two
  // results are separate states, so a hidden answer can never land in the pane
  // of the other mode.
  const changeMode = (next: Mode) => {
    if (next === mode) return;
    setMode(next);
    setCopied(false);
    setCopiedPart(null);
  };

  // Swap reverses the translation on screen: its text becomes the new source and
  // its language the new source language. The direction lives in this window's
  // state only — the saved Target is never written.
  const swap = () => {
    if (mode !== "translate") return;
    if (result.phase !== "translated" || !settings) return;
    const next = swapDirection(settings.language_pair, result.translation);
    setText(result.translation.text);
    setDirection(next);
    void runTranslation(result.translation.text, next);
  };

  const copy = () => {
    if (result.phase !== "translated") return;
    const translation = result.translation.text;
    ipc
      .copyText(translation)
      .then(() => setCopied(true))
      .catch(() => {
        // Nothing was copied, so the button must not claim otherwise.
      });
  };

  // Each Improve section copies only its own text: the corrected and the better
  // version are separate things the user may want in different places.
  const copyImprovement = (part: ImprovePart) => {
    if (improvement.phase !== "improving" && improvement.phase !== "improved") {
      return;
    }
    const value = improvement[part];
    if (value.length === 0) return;
    ipc
      .copyText(value)
      .then(() => setCopiedPart(part))
      .catch(() => {
        // Nothing was copied; leave the button saying Copy.
      });
  };

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

  // Improve streams as events rather than as the answer to its command. Every
  // event carries the request id the view chose, and the reducer drops anything
  // that is no longer the request in flight, so a superseded stream can never
  // land in this pane.
  useEffect(() => {
    const stops = [
      onTranslationChunk((payload) => {
        // `part` is present only for Improve; a chunk without it belongs to a
        // different pane's stream.
        if (!payload.part) return;
        setImprovement((state) =>
          reduceImprovement(state, {
            type: "chunk",
            requestId: Number(payload.request_id),
            part: payload.part as ImprovePart,
            text: payload.text,
          }),
        );
      }),
      onTranslationDone((payload) => {
        const outcome = payload.outcome;
        if (!("improvement" in outcome)) return;
        const { improvement } = outcome;
        setImprovement((state) =>
          reduceImprovement(state, {
            type: "done",
            requestId: Number(payload.request_id),
            improvement,
          }),
        );
      }),
      onTranslationError((payload) => {
        setImprovement((state) =>
          reduceImprovement(state, {
            type: "failed",
            requestId: Number(payload.request_id),
            error: payload.error,
          }),
        );
      }),
    ];

    return () => {
      for (const stop of stops) {
        void stop.then((unlisten) => unlisten()).catch(() => {
          // The listener never attached, so there is nothing to stop.
        });
      }
    };
  }, []);

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

  const canSwap = settings !== null && result.phase === "translated";
  const canCopy = result.phase === "translated";

  const isImprove = mode === "improve";
  const actionLabel = isImprove ? "Improve" : "Translate";
  // The two parts only exist once a request has started; before that, and after
  // a failure, the pane shows words instead.
  const improveParts =
    improvement.phase === "improving" || improvement.phase === "improved"
      ? improvement
      : null;

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

      <div className="mode-switch" role="tablist" aria-label="Mode">
        {(["translate", "improve"] as Mode[]).map((option) => (
          <button
            key={option}
            role="tab"
            aria-selected={mode === option}
            className={mode === option ? "tab active" : "tab"}
            onClick={() => changeMode(option)}
          >
            {option === "translate" ? "Translate" : "Improve"}
          </button>
        ))}
      </div>

      <div className="panes">
        <section className="pane">
          <h2>Source</h2>
          <textarea
            aria-label={isImprove ? "English text to improve" : "Text to translate"}
            className="source-input"
            value={text}
            onChange={(event) => changeText(event.target.value)}
            onKeyDown={(event) => {
              // Enter acts; Shift+Enter is the newline the editor needs.
              if (event.key === "Enter" && !event.shiftKey) {
                event.preventDefault();
                translate();
              }
            }}
            placeholder={
              isImprove
                ? "Type an English sentence to correct and improve."
                : `Select text in any app and press ${translateShortcut}, or type here.`
            }
            spellCheck={false}
            autoFocus
          />
          <div className="pane-actions">
            <button className="primary" onClick={translate}>
              {actionLabel}
            </button>
            <span className="muted small">
              {text.trim().length === 0
                ? isImprove
                  ? "Nothing to improve — the window opens empty."
                  : "Nothing selected — the window opens empty."
                : `${text.length} characters · Enter ${isImprove ? "improves" : "translates"}, Shift+Enter adds a line`}
            </span>
          </div>
        </section>

        <section className="pane">
          <div className="pane-head">
            <h2>{isImprove ? "Improved" : "Translation"}</h2>
            {!isImprove && result.phase === "translated" ? (
              <span className="provider">
                via {providerLabel(result.translation.provider)}
              </span>
            ) : null}
          </div>

          {isImprove ? (
            <>
              {improvement.phase === "empty" ? (
                <p className="empty">
                  Press Enter to correct the sentence and make it more natural.
                </p>
              ) : null}

              {improveParts ? (
                <div className="improve-parts">
                  {IMPROVE_PARTS.map((part) => {
                    const value = improveParts[part];
                    return (
                      <section className="improve-part" key={part}>
                        <div className="improve-part-head">
                          <h3>{partLabel(part)}</h3>
                          <button
                            onClick={() => copyImprovement(part)}
                            disabled={value.length === 0}
                          >
                            {copiedPart === part ? "Copied" : "Copy"}
                          </button>
                        </div>
                        <p className="result-text" aria-label={partLabel(part)}>
                          {value}
                        </p>
                      </section>
                    );
                  })}
                </div>
              ) : null}

              {improvement.phase === "failed" ? (
                <div className={`result-failure ${improvement.kind}`} role="alert">
                  <strong>{improvement.title}</strong>
                  <span className="muted">{improvement.detail}</span>
                </div>
              ) : null}
            </>
          ) : (
            <>
              {result.phase === "empty" ? (
                <p className="empty">
                  Press Enter to translate. The result can be selected in part.
                </p>
              ) : null}

              {result.phase === "translating" ? (
                <p className="empty" role="status">
                  Translating…
                </p>
              ) : null}

              {result.phase === "translated" ? (
                <p className="result-text" aria-label="Translation">
                  {result.translation.text}
                </p>
              ) : null}

              {result.phase === "failed" ? (
                <div className={`result-failure ${result.kind}`} role="alert">
                  <strong>{result.title}</strong>
                  <span className="muted">{result.detail}</span>
                </div>
              ) : null}
            </>
          )}

          {isImprove ? null : (
            <div className="pane-actions">
              <button onClick={swap} disabled={!canSwap}>
                Swap
              </button>
              <button onClick={copy} disabled={!canCopy}>
                {copied ? "Copied" : "Copy"}
              </button>
            </div>
          )}
        </section>
      </div>

      {settings ? (
        <details className="provider-details">
          <summary>Provider keys</summary>
          <ProviderSettings settings={settings} onSettingsSaved={setSettings} />
        </details>
      ) : null}

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
