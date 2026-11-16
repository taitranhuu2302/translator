import { Loader2 } from "lucide-react";
import {
  useCallback,
  useEffect,
  useReducer,
  useRef,
  useState,
} from "react";

import { Button } from "@/components/ui/button";

import { beginSession } from "./answerToken";
import { providerLabel } from "./format";
import { ipc, onPopupPrefill, type EngineError } from "./ipc";
import { performNoticeAction } from "./noticeActions";
import { NoticeView } from "./NoticeView";
import { OPENING, reduce } from "./popupState";

/**
 * The room the card leaves inside the transparent window for its shadow, and
 * the card's own border. The page adds both when it reports its height, because
 * the shell sizes the window, not the card.
 */
const OUTER_PADDING = 10;
const CARD_BORDER = 1;

/**
 * The Quick Translate panel's page. It is shown before a translation exists, so
 * the first thing it draws is "translating"; the answer then replaces it.
 *
 * The window is transparent and exactly as tall as the card, within the bounds
 * the shell clamps to. So the page measures what it drew and reports it, and
 * every control here is a plain element: a portal dropdown would be clipped by
 * a window this small.
 */
export function Popup() {
  const [state, dispatch] = useReducer(reduce, OPENING);
  const [copied, setCopied] = useState(false);
  const hadFocus = useRef(false);
  const dismissed = useRef(false);
  /// Set while the shell has taken the panel away to run Replace. The blur that
  /// hiding causes is ours, not the user's, and must not dismiss the answer we
  /// may still need to show a failure on.
  const replacing = useRef(false);

  const header = useRef<HTMLElement | null>(null);
  const content = useRef<HTMLDivElement | null>(null);
  const footer = useRef<HTMLElement | null>(null);
  const reported = useRef(0);

  const dismiss = useCallback(() => {
    if (dismissed.current) return;
    dismissed.current = true;
    dispatch({ type: "dismissed" });
    void ipc.hidePopup().catch(() => {
      // The panel may already be gone; nothing else to do.
    });
  }, []);

  // The card is what the user sees; the window is fitted around it. Header and
  // footer never change height, but the content does — on every phase of the
  // answer — so all three are observed and one number is reported.
  //
  // There is no dependency list on purpose: a dismissed popup draws no card, so
  // the next opening has to attach the observer to elements that did not exist
  // at mount.
  useEffect(() => {
    const body = content.current;
    if (!body) return;

    const report = () => {
      const height = Math.ceil(
        OUTER_PADDING * 2 +
          CARD_BORDER * 2 +
          (header.current?.offsetHeight ?? 0) +
          body.getBoundingClientRect().height +
          (footer.current?.offsetHeight ?? 0),
      );
      // A resize never changes what the card draws, so this cannot settle into
      // a loop; the guard only keeps identical reports off the IPC channel.
      if (height === reported.current) return;
      reported.current = height;
      void ipc.setPopupHeight(height).catch(() => {
        // The card keeps the height it had; the shell decides every bound.
      });
    };

    const observer = new ResizeObserver(report);
    observer.observe(body);
    if (header.current) observer.observe(header.current);
    if (footer.current) observer.observe(footer.current);
    report();
    return () => observer.disconnect();
  });

  // The prefill was captured before the panel appeared. Claim it on mount and
  // again whenever a later shortcut reopens the panel.
  useEffect(() => {
    let cancelled = false;

    const claim = async () => {
      let prefill;
      try {
        prefill = await ipc.takePopupPrefill();
      } catch {
        return;
      }
      // On the first show the mount claim and the `popup-prefill` listener race
      // for the same prefill; whichever loses sees null and stops here.
      if (cancelled || dismissed.current || !prefill) return;

      // Every open is its own session, so an answer still in flight from the
      // previous open carries a token the reducer has already retired.
      const token = beginSession().issue();
      // A prefill always starts a fresh session, so this also reopens a popup
      // that was dismissed or still showing the previous answer.
      dispatch({ type: "prefill", prefill, token });
      if (prefill === "nothing_selected" || "failure" in prefill) return;

      try {
        const outcome = await ipc.translateText(prefill.selection.text, "translate");
        if (cancelled) return;
        if ("translation" in outcome) {
          dispatch({ type: "translated", translation: outcome.translation, token });
        }
      } catch (error) {
        if (cancelled) return;
        dispatch({ type: "failed", error: error as EngineError, token });
      }
    };

    // A later shortcut reuses this same panel. Re-arm it and claim the new
    // prefill; the reducer's token rule makes anything still in flight
    // ignorable.
    const unlisten = onPopupPrefill(() => {
      dismissed.current = false;
      replacing.current = false;
      setCopied(false);
      void claim();
    });
    void claim();

    return () => {
      cancelled = true;
      void unlisten.then((stop) => stop()).catch(() => {
        // The listener never attached, so there is nothing to stop.
      });
    };
  }, []);

  // Enter replaces the Selection with the translation on screen. The shell hides
  // the panel before writing, so the source app receives the paste fallback; if
  // the write is refused the panel comes back and the reason is shown inline.
  const replace = useCallback(async () => {
    if (state.phase !== "translated") return;
    replacing.current = true;
    dispatch({ type: "replace_requested" });
    try {
      await ipc.replaceWithTranslation(state.source, state.translation);
      dismissed.current = true;
      dispatch({ type: "dismissed" });
    } catch (error) {
      replacing.current = false;
      dispatch({ type: "replace_failed", error: error as EngineError });
    }
  }, [state]);

  // Escape dismisses. Losing key to another app dismisses too; the source app
  // then owns the keyboard again with its caret where it was.
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        dismiss();
        return;
      }
      if (event.key === "Enter") {
        event.preventDefault();
        void replace();
      }
    };
    const onFocus = () => {
      hadFocus.current = true;
    };
    const onBlur = () => {
      // A blur caused by Replace handing the keyboard back is not a dismissal.
      if (hadFocus.current && !replacing.current) dismiss();
    };

    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("focus", onFocus);
    window.addEventListener("blur", onBlur);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("focus", onFocus);
      window.removeEventListener("blur", onBlur);
    };
  }, [dismiss, replace]);

  const copy = async () => {
    if (state.phase !== "translated" && state.phase !== "replace_failed") return;
    try {
      await ipc.copyText(state.translation.text);
      setCopied(true);
    } catch {
      setCopied(false);
    }
  };

  if (state.phase === "dismissed") {
    return null;
  }

  // The answer stays on screen while it is being written and after a refused
  // write, so the user can still Copy what they saw.
  const answer =
    state.phase === "translated" ||
    state.phase === "replacing" ||
    state.phase === "replace_failed"
      ? state.translation
      : null;

  return (
    <main className="flex h-full flex-col p-2.5">
      <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border border-border/40 bg-card text-card-foreground shadow-lg">
        <header
          ref={header}
          className="flex h-9 shrink-0 items-center gap-2 border-b border-border/40 px-3"
        >
          <span className="text-[0.6875rem] font-medium tracking-[0.08em] text-muted-foreground uppercase">
            Quick Translate
          </span>
          {answer ? (
            <span className="ml-auto truncate text-[0.6875rem] text-muted-foreground">
              {providerLabel(answer.provider)}
            </span>
          ) : null}
        </header>

        {/* The scroll region carries no padding of its own: the measured box is
            the one that is padded, so its height is the whole of what the card
            needs for its text. */}
        <div className="min-h-0 flex-1 overflow-y-auto">
          <div ref={content} className="flex flex-col gap-2.5 px-3.5 py-3">
            {"source" in state ? (
              <p className="line-clamp-2 text-xs leading-relaxed text-muted-foreground">
                {state.source}
              </p>
            ) : null}

            {state.phase === "opening" ? (
              <StatusLine label="Reading the selection…" />
            ) : null}

            {state.phase === "translating" ? (
              <StatusLine label="Translating…" />
            ) : null}

            {answer ? (
              <p className="text-[0.9375rem] leading-relaxed whitespace-pre-wrap text-foreground/90">
                {answer.text}
              </p>
            ) : null}

            {state.phase === "replacing" ? (
              <StatusLine label="Replacing…" />
            ) : null}

            {state.phase === "replace_failed" ||
            state.phase === "nothing_selected" ||
            state.phase === "failed" ? (
              <NoticeView notice={state} onAction={performNoticeAction} />
            ) : null}
          </div>
        </div>

        <footer
          ref={footer}
          className="flex h-8 shrink-0 items-center gap-2 border-t border-border/40 px-3 text-[0.6875rem] text-muted-foreground"
        >
          <span className="truncate">
            {answer ? "Enter replaces · Esc dismisses" : "Esc dismisses"}
          </span>
          {answer ? (
            <Button
              size="xs"
              variant="ghost"
              className="ml-auto h-6 shrink-0 px-2 text-[0.6875rem] text-foreground/80 hover:text-foreground"
              onClick={() => void copy()}
            >
              {copied ? "Copied" : "Copy"}
            </Button>
          ) : null}
        </footer>
      </div>
    </main>
  );
}

/** One line of progress. The spinner is decoration; the text carries the state. */
function StatusLine({ label }: { label: string }) {
  return (
    <p
      role="status"
      className="flex items-center gap-2 text-sm text-muted-foreground"
    >
      <Loader2 aria-hidden="true" className="size-3.5 animate-spin" />
      {label}
    </p>
  );
}
