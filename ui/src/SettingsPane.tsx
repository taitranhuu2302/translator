import { useCallback, useEffect, useState, type ReactNode } from "react";

import { Button } from "@/components/ui/button";
import { Kbd } from "@/components/ui/kbd";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";

import { formatShortcut } from "./format";
import {
  ipc,
  type Settings,
  type ShortcutAction,
  type ShortcutStatus,
} from "./ipc";
import { ProviderSettings } from "./ProviderSettings";
import {
  directionSentence,
  languageName,
  OFFERED_LANGUAGES,
} from "./settingsView";
import { captureBinding, type KeyPress } from "./shortcuts";
import { settingsWriter } from "./settingsWrite";

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

  // The switch shows what macOS actually has, not what the settings file
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
        // The actual state stays unknown; the switch keeps its last value.
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
        const outcome = await settingsWriter.change({
          kind: "rebind",
          action,
          binding,
        });
        if (outcome.mirror === null) {
          setProblems((current) => ({ ...current, [action]: outcome.problem }));
          return;
        }
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
        onSettingsSaved(outcome.mirror.settings);
      } finally {
        setBusy(false);
      }
    },
    [onSettingsSaved],
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
    setBusy(true);
    setPairNotice(null);
    try {
      const outcome = await settingsWriter.change({ kind: "target", target });
      if (outcome.mirror === null) {
        setPairNotice(outcome.problem);
        return;
      }
      onSettingsSaved(outcome.mirror.settings);
      setPairNotice(outcome.notice);
    } finally {
      setBusy(false);
    }
  };

  const reset = async () => {
    setBusy(true);
    setResetNotice(null);
    setResetProblems([]);
    try {
      const outcome = await settingsWriter.change({ kind: "reset" });
      setResetProblems(outcome.refusals);
      if (outcome.mirror === null) {
        setResetNotice(outcome.problem);
        return;
      }
      onSettingsSaved(outcome.mirror.settings);
      setProblems({});
      setResetNotice(outcome.notice);
      setConfirmReset(false);
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
  const knownTarget = OFFERED_LANGUAGES.some(
    (language) => language.code === settings.language_pair.target,
  );

  return (
    <div className="flex flex-col gap-7 px-4 py-5">
      <Section title="Language Pair">
        <div className="flex flex-col gap-2">
          <Label htmlFor="target-language">Target</Label>
          <Select
            value={settings.language_pair.target}
            onValueChange={(value) => {
              void savePair(value as string);
            }}
            disabled={busy}
          >
            <SelectTrigger id="target-language" className="w-full">
              {/* The trigger names the language, not its code: Base UI prints
                  the raw value unless told how to read it. */}
              <SelectValue>
                {(value) => languageName(String(value ?? ""))}
              </SelectValue>
            </SelectTrigger>
            <SelectContent>
              {OFFERED_LANGUAGES.map((language) => (
                <SelectItem key={language.code} value={language.code}>
                  {language.name}
                </SelectItem>
              ))}
              {/* A Target saved outside this view still shows instead of a blank
                  select; choosing an offered language replaces it. */}
              {knownTarget ? null : (
                <SelectItem value={settings.language_pair.target}>
                  {languageName(settings.language_pair.target)}
                </SelectItem>
              )}
            </SelectContent>
          </Select>
          <p className="text-xs text-muted-foreground" aria-live="polite">
            {directionSentence(settings.language_pair)}
          </p>
          {pairNotice ? (
            <p className="text-xs text-muted-foreground">{pairNotice}</p>
          ) : null}
        </div>
      </Section>

      <Section title="Startup">
        <div className="flex items-center justify-between gap-3">
          <Label htmlFor="start-at-login" className="font-normal">
            Start Neris at login
          </Label>
          <Switch
            id="start-at-login"
            checked={loginEnabled ?? false}
            disabled={busy || loginEnabled === null}
            onCheckedChange={(checked) => {
              void saveStartAtLogin(checked);
            }}
          />
        </div>
        {loginProblem ? (
          <p className="text-xs text-destructive" role="alert">
            {loginProblem}
          </p>
        ) : null}
      </Section>

      <Section title="Shortcuts">
        {problemConflicts.length > 0 ? (
          <div
            role="status"
            className="rounded-lg border border-border/70 bg-card/60 px-3 py-2.5"
          >
            <p className="text-xs text-foreground">
              These shortcuts were not live when Neris started:
            </p>
            <ul className="mt-1.5 flex flex-col gap-1">
              {problemConflicts.map((status) => (
                <li className="text-xs text-destructive" key={status.action}>
                  <strong className="font-medium">
                    {actionLabel(status.action)}
                  </strong>{" "}
                  ({formatShortcut(status.binding)}): {status.problem}
                </li>
              ))}
            </ul>
          </div>
        ) : null}

        <p className="text-xs leading-relaxed text-muted-foreground">
          Choose one, then press the combination you want. At least one of Ctrl,
          Alt, Cmd or Shift is required.
        </p>

        <div className="flex flex-col gap-2">
          {ACTIONS.map(({ action, label, description }) => (
            <div
              key={action}
              className="flex flex-col gap-1.5 rounded-lg border border-border/60 bg-card/50 px-3 py-2.5"
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <p className="text-sm text-foreground">{label}</p>
                  <p className="text-xs leading-relaxed text-muted-foreground">
                    {description}
                  </p>
                </div>
                <div className="flex shrink-0 items-center gap-2">
                  {recording === action ? (
                    <span className="text-xs text-primary">
                      Press the combination…
                    </span>
                  ) : (
                    <Kbd>{formatShortcut(settings.shortcuts[action])}</Kbd>
                  )}
                  <Button
                    size="xs"
                    variant="outline"
                    onClick={() =>
                      setRecording(recording === action ? null : action)
                    }
                    disabled={
                      busy || (recording !== null && recording !== action)
                    }
                  >
                    {recording === action ? "Listening…" : "Change"}
                  </Button>
                </div>
              </div>
              {problems[action] ? (
                <p className="text-xs text-destructive" role="alert">
                  {problems[action]}
                </p>
              ) : null}
            </div>
          ))}
        </div>
      </Section>

      <Section title="Providers">
        <ProviderSettings
          settings={settings}
          onSettingsSaved={onSettingsSaved}
        />
      </Section>

      <Section title="Reset">
        <p className="text-xs leading-relaxed text-muted-foreground">
          Puts back the shipped shortcuts and English ⇄ Vietnamese. History and
          API keys are kept.
        </p>
        {confirmReset ? (
          <div className="flex flex-wrap items-center gap-2">
            <Button variant="destructive" onClick={reset} disabled={busy}>
              Reset shortcuts and language pair
            </Button>
            <Button
              variant="ghost"
              onClick={() => setConfirmReset(false)}
              disabled={busy}
            >
              Keep my settings
            </Button>
          </div>
        ) : (
          <div>
            <Button
              variant="outline"
              onClick={() => setConfirmReset(true)}
              disabled={busy}
            >
              Reset shortcuts and language pair…
            </Button>
          </div>
        )}
        {resetNotice ? (
          <p className="text-xs text-muted-foreground">{resetNotice}</p>
        ) : null}
        {resetProblems.length > 0 ? (
          <ul className="flex flex-col gap-1">
            {resetProblems.map((problem) => (
              <li className="text-xs text-destructive" key={problem}>
                {problem}
              </li>
            ))}
          </ul>
        ) : null}
      </Section>
    </div>
  );
}

/** One titled block of the sheet. The heading is a label, not a landmark: the
 * sheet already has one title. */
function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-3">
      <h3 className="text-[0.6875rem] font-medium tracking-[0.09em] text-muted-foreground uppercase">
        {title}
      </h3>
      {children}
    </section>
  );
}

/** The name the Settings rows and the launch warning share for an action. */
function actionLabel(action: ShortcutAction): string {
  return ACTIONS.find((entry) => entry.action === action)?.label ?? action;
}
