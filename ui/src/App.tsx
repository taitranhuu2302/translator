import { useCallback, useEffect, useRef, useState } from "react";

import { AppShell } from "@/components/AppShell";
import { HistorySheet } from "@/components/HistorySheet";
import { SettingsSheet } from "@/components/SettingsSheet";
import { TranslatePanes } from "@/components/TranslatePanes";

import { beginSession, type AnswerSession } from "./answerToken";
import { formatShortcut } from "./format";
import {
  EMPTY_IMPROVEMENT,
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
  type HistoryEntry,
  type ImprovePart,
  type Mode,
  type Prefill,
  type Settings,
} from "./ipc";
import { noticeFor, noticeForSettingsFailure, type Notice } from "./notice";
import { performNoticeAction } from "./noticeActions";
import { CaptureNoticeView, NoticeView } from "./NoticeView";
import { Onboarding } from "./Onboarding";
import {
  effectiveDirection,
  EMPTY_RESULT,
  reduceResult,
  swapDirection,
  type TranslateResult,
} from "./translateWindow";

const FALLBACK_SHORTCUT = "Alt+Shift+E";

/** How many recent entries the History sheet loads. It is a cap, not a page
 * size: v1 shows recent history, and the search narrows what was loaded. */
const HISTORY_LIMIT = 500;

/**
 * The window: two panes that stay put, and two sheets that slide over them.
 *
 * History and Settings are no longer places the window goes — they are panels
 * beside the work, so a trip to either leaves both panes exactly as they were.
 * The state machine below is unchanged; only where it is drawn moved.
 */
export function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [settingsError, setSettingsError] = useState<string | null>(null);

  /// null until the shell answers, so a missing grant is never assumed.
  const [granted, setGranted] = useState<boolean | null>(null);

  const [text, setText] = useState("");
  const [failure, setFailure] = useState<Notice | null>(null);
  const [mode, setMode] = useState<Mode>("translate");

  const [historyOpen, setHistoryOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);

  // History is read once when the window opens and again whenever its sheet is
  // opened. The query filters these entries locally, so typing never asks the
  // engine for anything.
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [historyQuery, setHistoryQuery] = useState("");
  const [historyError, setHistoryError] = useState<string | null>(null);

  // The result pane, the direction this window translates in (null means the
  // saved Language Pair), and whether the Copy button has just been used.
  const [result, setResult] = useState<TranslateResult>(EMPTY_RESULT);
  const [direction, setDirection] = useState<Direction | null>(null);
  const [copied, setCopied] = useState(false);

  // The direction the next Translate would run in, as the local Detection reads
  // the text being typed. It only labels the Source pane: submitting resolves
  // the direction again in the engine.
  const [detectedDirection, setDetectedDirection] = useState<Direction | null>(
    null,
  );
  const detection = useRef(0);

  // Improve keeps its own pane: two labelled parts that fill in as the model
  // writes. `copiedPart` says which section's Copy was last used.
  const [improvement, setImprovement] = useState<ImproveResult>(
    EMPTY_IMPROVEMENT,
  );
  const [copiedPart, setCopiedPart] = useState<ImprovePart | null>(null);

  // Every attempt this window makes is issued a token from the current
  // session. A new opening starts a new session, retiring every answer still in
  // flight; the reducers compare each answer's token and drop anything older.
  const tokens = useRef<AnswerSession | null>(null);

  const applyPrefill = useCallback((prefill: Prefill | null) => {
    if (!prefill) return;
    if ("selection" in prefill) {
      tokens.current = beginSession();
      setText(prefill.selection.text);
      setFailure(null);
      setResult((state) => reduceResult(state, { type: "cleared" }));
      setImprovement((state) => reduceImprovement(state, { type: "cleared" }));
      // A new opening starts in Translate, from the saved pair again; Swap never
      // survives it, and neither sheet is left covering the new text.
      setMode("translate");
      setHistoryOpen(false);
      setSettingsOpen(false);
      setDirection(null);
      setCopied(false);
      setCopiedPart(null);
      return;
    }
    setFailure(noticeFor(prefill.failure));
  }, []);

  // The newest entries, unfiltered: the History sheet narrows them in memory.
  // A read that fails leaves the last list alone and says so.
  const loadHistory = useCallback(() => {
    ipc
      .searchHistory("", HISTORY_LIMIT)
      .then((entries) => {
        setHistory(entries);
        setHistoryError(null);
      })
      .catch(() => {
        setHistoryError("History could not be read.");
      });
  }, []);

  const runTranslation = useCallback(
    async (source: string, requested: Direction | null) => {
      setCopied(false);
      // The engine now enforces the blank rule; this check only improves the
      // affordance by clearing the pane instead of awaiting a refusal.
      if (source.trim().length === 0) {
        setResult((state) => reduceResult(state, { type: "cleared" }));
        return;
      }
      tokens.current ??= beginSession();
      const token = tokens.current.issue();
      setResult((state) => reduceResult(state, { type: "started", token }));
      try {
        const outcome = await ipc.translateText(source, "translate", requested);
        if ("translation" in outcome) {
          const translation = outcome.translation;
          setResult((state) =>
            reduceResult(state, { type: "translated", translation, token }),
          );
        }
      } catch (error) {
        setResult((state) =>
          reduceResult(state, {
            type: "failed",
            error: error as EngineError,
            token,
          }),
        );
      }
    },
    [],
  );

  // Improve streams through events rather than an answer to this promise: the
  // command returns once its worker starts. `done` is the only value the
  // provider strictly validated, so it replaces what the chunks showed.
  const runImprovement = useCallback(async (source: string) => {
    setCopiedPart(null);
    // The engine now enforces the blank rule; this check only improves the
    // affordance by clearing the pane instead of awaiting a refusal.
    if (source.trim().length === 0) {
      setImprovement((state) => reduceImprovement(state, { type: "cleared" }));
      return;
    }
    tokens.current ??= beginSession();
    const token = tokens.current.issue();
    setImprovement((state) =>
      reduceImprovement(state, { type: "started", token }),
    );
    try {
      await ipc.improveText(token, source);
    } catch (error) {
      // The source text is App's `text` state and is never touched here: a
      // failed Improve must leave what the user typed exactly as it was.
      setImprovement((state) =>
        reduceImprovement(state, {
          type: "failed",
          token,
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
    // An edit is new text, so the next Translate detects its direction again.
    // A Swap applies only to the text it produced; carrying it across an edit
    // would translate somewhere the user did not ask for.
    setDirection(null);
    // Emptying the input leaves both panes as they are: a result stays readable
    // until the next submit. Submitting empty text is what clears the pane, and
    // it clears only the pane of the mode that was submitted.
  };

  // Switching between the two working modes keeps an in-flight stream alive in
  // its own pane: the two results are separate states, so a hidden answer can
  // never land in the pane of the other mode.
  const changeMode = (next: Mode) => {
    if (next === mode) return;
    setMode(next);
    setCopied(false);
    setCopiedPart(null);
  };

  // The Source pane's live label. Every keystroke issues a new Detection and
  // retires the answer still in flight, so an older guess can never overwrite a
  // newer one. Improve has no direction, and before the settings arrive there is
  // no pair to read the text against.
  useEffect(() => {
    const attempt = ++detection.current;
    if (mode !== "translate" || !settings || text.trim().length === 0) {
      setDetectedDirection(null);
      return;
    }
    ipc
      .detectDirection(text, settings.language_pair)
      .then((direction) => {
        if (attempt === detection.current) setDetectedDirection(direction);
      })
      .catch(() => {
        if (attempt === detection.current) setDetectedDirection(null);
      });
  }, [mode, settings, text]);

  const openHistory = () => {
    setHistoryOpen(true);
    loadHistory();
  };

  // A stored entry is every part of a result: choosing it puts its source back
  // in the input, opens the mode it was made in, restores a known direction, and
  // fills that pane with the answer the engine shaped from the row. No provider
  // is called and no new row is written.
  const openHistoryEntry = async (entry: HistoryEntry) => {
    setHistoryOpen(false);
    setMode(entry.mode);
    setText(entry.source);
    setFailure(null);
    setCopied(false);
    setCopiedPart(null);
    setDirection(entry.mode === "translate" ? entry.direction : null);
    try {
      const outcome = await ipc.reopenHistoryEntry(entry);
      if ("translation" in outcome) {
        setResult((state) =>
          reduceResult(state, {
            type: "restored",
            translation: outcome.translation,
          }),
        );
      } else {
        setImprovement((state) =>
          reduceImprovement(state, {
            type: "restored",
            improvement: outcome.improvement,
          }),
        );
      }
    } catch {
      setHistoryError("That entry could not be reopened.");
    }
  };

  // Swap reverses the translation on screen: its text becomes the new source and
  // its language the new source language. The direction lives in this window's
  // state only — the saved Target is never written.
  const swap = () => {
    if (mode !== "translate") return;
    if (result.phase !== "translated") return;
    const next = swapDirection(result.translation);
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
          // Leave the state unknown: a grant that cannot be read back must not
          // hide the notice that explains how to make one.
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
      void unlisten
        .then((stop) => stop())
        .catch(() => {
          // The listener never attached, so there is nothing to stop.
        });
    };
  }, [applyPrefill]);

  // Improve streams as events rather than as the answer to its command. The IPC
  // seam hands each event back with the token this window issued, and the
  // reducer drops anything that is no longer the attempt in flight, so a
  // superseded stream can never land in this pane.
  useEffect(() => {
    const stops = [
      onTranslationChunk(({ token, part, text }) => {
        // `part` is present only for Improve; a chunk without it belongs to a
        // different pane's stream.
        if (!part) return;
        setImprovement((state) =>
          reduceImprovement(state, { type: "chunk", token, part, text }),
        );
      }),
      onTranslationDone(({ token, outcome }) => {
        if (!("improvement" in outcome)) return;
        setImprovement((state) =>
          reduceImprovement(state, {
            type: "done",
            token,
            improvement: outcome.improvement,
          }),
        );
      }),
      onTranslationError(({ token, error }) => {
        setImprovement((state) =>
          reduceImprovement(state, { type: "failed", token, error }),
        );
      }),
    ];

    return () => {
      for (const stop of stops) {
        void stop
          .then((unlisten) => unlisten())
          .catch(() => {
            // The listener never attached, so there is nothing to stop.
          });
      }
    };
  }, []);

  // History survives restarts in SQLite, so the list is read at launch too, not
  // only when its sheet is opened.
  useEffect(() => {
    loadHistory();
  }, [loadHistory]);

  const translateShortcut = formatShortcut(
    settings?.shortcuts.translate_window ?? FALLBACK_SHORTCUT,
  );
  const replaceShortcut = formatShortcut(
    settings?.shortcuts.replace ?? "Alt+Shift+R",
  );

  const finishOnboarding = async () => {
    setSettings(await ipc.completeOnboarding());
  };

  const canSwap = result.phase === "translated";
  const liveDirection = effectiveDirection(direction, detectedDirection);

  return (
    <AppShell
      mode={mode}
      onModeChange={changeMode}
      onOpenHistory={openHistory}
      onOpenSettings={() => setSettingsOpen(true)}
      translateShortcut={translateShortcut}
      replaceShortcut={replaceShortcut}
    >
      {failure ? (
        <div className="shrink-0 px-4 pt-4">
          <CaptureNoticeView
            notice={failure}
            granted={granted}
            shortcut={translateShortcut}
            onAction={performNoticeAction}
          />
        </div>
      ) : settingsError ? (
        <div className="shrink-0 px-4 pt-4">
          <NoticeView notice={noticeForSettingsFailure(settingsError)} />
        </div>
      ) : null}

      <TranslatePanes
        mode={mode}
        text={text}
        onTextChange={changeText}
        onSubmit={translate}
        result={result}
        liveDirection={liveDirection}
        improvement={improvement}
        copied={copied}
        copiedPart={copiedPart}
        onCopy={copy}
        onCopyPart={copyImprovement}
        onSwap={swap}
        canSwap={canSwap}
        shortcut={translateShortcut}
      />

      <HistorySheet
        open={historyOpen}
        onOpenChange={setHistoryOpen}
        entries={history}
        query={historyQuery}
        error={historyError}
        onQueryChange={setHistoryQuery}
        onOpen={(entry) => {
          void openHistoryEntry(entry);
        }}
      />

      <SettingsSheet
        open={settingsOpen}
        onOpenChange={setSettingsOpen}
        settings={settings}
        error={settingsError}
        onSettingsSaved={setSettings}
      />

      {settings && !settings.onboarding_seen ? (
        <Onboarding
          shortcuts={settings.shortcuts}
          granted={granted === true}
          onDone={finishOnboarding}
        />
      ) : null}
    </AppShell>
  );
}
