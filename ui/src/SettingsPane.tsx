import { useCallback, useEffect, useState } from "react";

import { formatShortcut } from "./format";
import {
  ipc,
  type ResetOutcome,
  type Settings,
  type ShortcutAction,
  type ShortcutStatus,
} from "./ipc";
import { ProviderSettings } from "./ProviderSettings";
import {
  directionSentence,
  languageName,
  OFFERED_LANGUAGES,
  withTarget,
} from "./settingsView";
import { captureBinding, withShortcut, type KeyPress } from "./shortcuts";

/** What a rebind says when the shell could not carry it out at all, rather
 * than the engine answering with its own typed refusal. */
const REBIND_FAILED = "The shortcut could not be changed.";

/** The four actions, named the way the engine names them in its messages. */
const ACTIONS: { action: ShortcutAction; label: string; description: string }[] =
  [
    {
      action: "quick_translate",
      label: "Quick Translate",
      description: "Shows a translation of the Selection in a popup.",
    },
    {
      action: "replace",
      label: "Replace",
      description: "Replaces the Selection with its translation.",
    },
    {
      action: "read_aloud",
      label: "Read Aloud",
      description: "Speaks the Selection in its own language.",
    },
    {
      action: "translate_window",
      label: "Translate Window",
      description: "Opens this window with the Selection.",
    },
  ];

/**
 * Everything the Settings surface changes: one Target for the Language Pair, four
 * global shortcuts captured by pressing them, the provider rows, and the reset.
 * Every write goes through the engine's settings store; a change the engine
 * refuses is shown where it was made and the saved value stays as it was.
 */
export function SettingsPane({
  settings,
  onSettingsSaved,
}: {
  settings: Settings;
  onSettingsSaved: (settings: Settings) => void;
}) {
  const [recording, setRecording] = useState<ShortcutAction | null>(null);
  const [problems, setProblems] = useState<
    Partial<Record<ShortcutAction, string>>
  >({});
  const [pairNotice, setPairNotice] = useState<string | null>(null);
  const [resetNotice, setResetNotice] = useState<string | null>(null);
  const [resetProblems, setResetProblems] = useState<string[]>([]);
  const [confirmReset, setConfirmReset] = useState(false);
  const [busy, setBusy] = useState(false);
  const [loginEnabled, setLoginEnabled] = useState<boolean | null>(null);
  const [loginProblem, setLoginProblem] = useState<string | null>(null);
  const [launchConflicts, setLaunchConflicts] = useState<ShortcutStatus[]>([]);

  // The shortcuts macOS refused at launch are not a per-row problem until the
  // user tries that row; they are fetched once so the first painted list says
  // which actions are not working.
  const loadConflicts = useCallback(async () => {
    try {
      setLaunchConflicts(await ipc.shortcutConflicts());
    } catch {
      // The launch check could not be read; the rows still work as they are.
    }
  }, []);

  useEffect(() => {
    void loadConflicts();
  }, [loadConflicts]);

  // The checkbox shows what macOS actually has, not what the settings file
  // remembers: a login item the user removed by hand must not read as on.
  useEffect(() => {
    let cancelled = false;
    ipc
      .startAtLoginEnabled()
      .then((enabled) => {
        if (!cancelled) setLoginEnabled(enabled);
      })
      .catch(() => {
        if (!cancelled) {
          setLoginProblem("The login item state could not be read.");
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const saveStartAtLogin = async (enabled: boolean) => {
    setBusy(true);
    setLoginProblem(null);
    try {
      const outcome = await ipc.setStartAtLogin(enabled);
      setLoginEnabled(outcome.enabled);
      // A failed read answers with defaults that are not the user's settings;
      // only a change that actually reached the file is safe to mirror here.
      if (!outcome.problem) onSettingsSaved(outcome.settings);
      if (outcome.problem) setLoginProblem(outcome.problem);
    } catch {
      setLoginProblem("The login item could not be changed.");
      // The command refused outright; ask again rather than assume a state.
      try {
        setLoginEnabled(await ipc.startAtLoginEnabled());
      } catch {
        // The actual state stays unknown; the checkbox keeps its last value.
      }
    } finally {
      setBusy(false);
    }
  };

  const applyRebind = useCallback(
    async (action: ShortcutAction, binding: string) => {
      setRecording(null);
      setBusy(true);
      try {
        const result = await ipc.rebindShortcut(action, binding);
        if (result.problem) {
          setProblems((current) => ({
            ...current,
            [action]: result.problem ?? REBIND_FAILED,
          }));
        } else {
          // Only a binding that is both live and saved enters the view: what is
          // shown and what relaunching will restore are the same thing.
          setProblems((current) => {
            const next = { ...current };
            delete next[action];
            return next;
          });
          // The shell drops this action's launch refusal when the rebind is
          // saved; mirror that instead of re-showing a refusal that is gone.
          setLaunchConflicts((current) =>
            current.filter((status) => status.action !== action),
          );
          onSettingsSaved(withShortcut(settings, action, result.binding));
        }
      } catch {
        setProblems((current) => ({
          ...current,
          [action]: REBIND_FAILED,
        }));
      } finally {
        setBusy(false);
      }
    },
    [onSettingsSaved, settings],
  );

  // Press-to-record: while an action is listening, the next combination is the
  // new binding. Modifier-only presses keep listening; Escape cancels. The
  // listener runs in the capture phase so a combination never also reaches the
  // window as a command.
  useEffect(() => {
    if (!recording) return;

    const onKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
      if (event.code === "Escape") {
        setRecording(null);
        return;
      }
      const binding = captureBinding(event as KeyPress);
      if (!binding) return;
      void applyRebind(recording, binding);
    };

    document.addEventListener("keydown", onKeyDown, true);
    return () => document.removeEventListener("keydown", onKeyDown, true);
  }, [recording, applyRebind]);

  const savePair = async (target: string) => {
    const next = {
      ...settings,
      language_pair: withTarget(settings.language_pair, target),
    };
    if (next.language_pair === settings.language_pair) return;
    setBusy(true);
    setPairNotice(null);
    try {
      await ipc.saveSettings(next);
      onSettingsSaved(next);
      setPairNotice("Language pair saved.");
    } catch {
      setPairNotice("The language pair could not be saved.");
    } finally {
      setBusy(false);
    }
  };

  const reset = async () => {
    setBusy(true);
    setResetNotice(null);
    setResetProblems([]);
    try {
      const outcome: ResetOutcome = await ipc.resetShortcutsAndLanguagePair();
      onSettingsSaved(outcome.settings);
      setProblems({});
      setResetProblems(refusals(outcome.bindings));
      setResetNotice(
        outcome.save_problem ??
          "Shortcuts and language pair put back to the shipped settings.",
      );
      setConfirmReset(false);
    } catch {
      setResetNotice("The shipped settings could not be restored.");
    } finally {
      setBusy(false);
      // A reset clears every launch refusal; read the list back rather than
      // assuming it, because the command may have failed before clearing.
      void loadConflicts();
    }
  };

  const problemConflicts = launchConflicts.filter(
    (status) => status.problem !== null,
  );

  return (
    <div className="panes single">
      <section className="pane settings-pane">
        <div className="pane-head">
          <h2>Settings</h2>
          <span className="muted small">Saved on this Mac, never synced</span>
        </div>

        <section className="settings-section">
          <h3 className="settings-title">Language Pair</h3>
          <label className="field">
            <span>Target</span>
            <select
              className="settings-select"
              value={settings.language_pair.target}
              onChange={(event) => {
                void savePair(event.target.value);
              }}
              disabled={busy}
            >
              {OFFERED_LANGUAGES.map((language) => (
                <option key={language.code} value={language.code}>
                  {language.name}
                </option>
              ))}
              {/* A Target saved outside this view still shows instead of a blank
                  select; choosing an offered language replaces it. */}
              {OFFERED_LANGUAGES.some(
                (language) => language.code === settings.language_pair.target,
              ) ? null : (
                <option value={settings.language_pair.target}>
                  {languageName(settings.language_pair.target)}
                </option>
              )}
            </select>
          </label>
          <p className="muted small" aria-live="polite">
            {directionSentence(settings.language_pair)}
          </p>
          {pairNotice ? <p className="muted small">{pairNotice}</p> : null}
        </section>

        <section className="settings-section">
          <h3 className="settings-title">Startup</h3>
          <label className="login-toggle">
            <input
              type="checkbox"
              checked={loginEnabled ?? false}
              disabled={busy || loginEnabled === null}
              onChange={(event) => {
                void saveStartAtLogin(event.target.checked);
              }}
            />
            <span>Start Translator at login</span>
          </label>
          {loginProblem ? (
            <p className="error small" role="alert">
              {loginProblem}
            </p>
          ) : null}
        </section>

        <section className="settings-section">
          <h3 className="settings-title">Shortcuts</h3>
          {problemConflicts.length > 0 ? (
            <div className="launch-warning" role="status">
              <p className="small">
                These shortcuts were not live when Translator started:
              </p>
              <ul className="reset-problems">
                {problemConflicts.map((status) => (
                  <li className="error small" key={status.action}>
                    <strong>{actionLabel(status.action)}</strong> (
                    {formatShortcut(status.binding)}): {status.problem}
                  </li>
                ))}
              </ul>
            </div>
          ) : null}
          <p className="muted small">
            Choose one, then press the combination you want. At least one of Ctrl,
            Alt, Cmd or Shift is required.
          </p>
          {ACTIONS.map(({ action, label, description }) => (
            <div className="binding-row" key={action}>
              <div className="binding-text">
                <strong>{label}</strong>
                <span className="muted small">{description}</span>
              </div>
              {recording === action ? (
                <span className="binding-capture">Press the combination…</span>
              ) : (
                <kbd>{formatShortcut(settings.shortcuts[action])}</kbd>
              )}
              <button
                onClick={() =>
                  setRecording(recording === action ? null : action)
                }
                disabled={busy || (recording !== null && recording !== action)}
              >
                {recording === action ? "Listening…" : "Change"}
              </button>
              {problems[action] ? (
                <p className="error small binding-problem" role="alert">
                  {problems[action]}
                </p>
              ) : null}
            </div>
          ))}
        </section>

        <section className="settings-section">
          <h3 className="settings-title">Providers</h3>
          <ProviderSettings
            settings={settings}
            onSettingsSaved={onSettingsSaved}
          />
        </section>

        <section className="settings-section">
          <h3 className="settings-title">Reset</h3>
          <p className="muted small">
            Puts back the shipped shortcuts and English ⇄ Vietnamese. History and
            API keys are kept.
          </p>
          {confirmReset ? (
            <div className="actions">
              <button className="danger" onClick={reset} disabled={busy}>
                Reset shortcuts and language pair
              </button>
              <button onClick={() => setConfirmReset(false)} disabled={busy}>
                Keep my settings
              </button>
            </div>
          ) : (
            <div className="actions">
              <button onClick={() => setConfirmReset(true)} disabled={busy}>
                Reset shortcuts and language pair…
              </button>
            </div>
          )}
          {resetNotice ? <p className="muted small">{resetNotice}</p> : null}
          {resetProblems.length > 0 ? (
            <ul className="reset-problems">
              {resetProblems.map((problem) => (
                <li className="error small" key={problem}>
                  {problem}
                </li>
              ))}
            </ul>
          ) : null}
        </section>
      </section>
    </div>
  );
}

/** macOS refusals happen one binding at a time; each one is reported, so the
 * user knows exactly which shortcut is not live. */
function refusals(bindings: ShortcutStatus[]): string[] {
  return bindings
    .filter((status) => status.problem !== null)
    .map((status) => status.problem ?? "");
}

/** The name the Settings rows and the launch warning share for an action. */
function actionLabel(action: ShortcutAction): string {
  return ACTIONS.find((entry) => entry.action === action)?.label ?? action;
}
