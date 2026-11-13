import { useCallback, useEffect, useReducer, useRef, useState } from "react";

import { ipc, onPopupPrefill, type EngineError } from "./ipc";
import { OPENING, providerLabel, reduce } from "./popupState";

/**
 * The Quick Translate panel's page. It is shown before a translation exists, so
 * the first thing it draws is "translating"; the answer then replaces it.
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
  /// Identifies the current translation, so a slow answer from a previous open
  /// cannot fill a popup that is now translating something else.
  const session = useRef(0);

  const dismiss = useCallback(() => {
    if (dismissed.current) return;
    dismissed.current = true;
    dispatch({ type: "dismissed" });
    void ipc.hidePopup().catch(() => {
      // The panel may already be gone; nothing else to do.
    });
  }, []);

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

      session.current += 1;
      const ours = session.current;
      // A prefill always starts a fresh session, so this also reopens a popup
      // that was dismissed or still showing the previous answer.
      dispatch({ type: "prefill", prefill });
      if (prefill === "nothing_selected" || "failure" in prefill) return;

      try {
        const outcome = await ipc.translateText(prefill.selection.text, "translate");
        if (cancelled || ours !== session.current) return;
        if ("translation" in outcome) {
          dispatch({ type: "translated", translation: outcome.translation });
        }
      } catch (error) {
        if (cancelled || ours !== session.current) return;
        dispatch({ type: "failed", error: error as EngineError });
      }
    };

    // A later shortcut reuses this same panel. Re-arm it and claim the new
    // prefill; the session counter makes anything still in flight ignorable.
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
    <main className="popup">
      <header className="popup-head">
        <h1 className="popup-title">Quick Translate</h1>
        {answer ? (
          <span className="popup-provider">{providerLabel(answer.provider)}</span>
        ) : null}
      </header>

      {"source" in state ? <p className="popup-source">{state.source}</p> : null}

      {state.phase === "opening" ? (
        <p className="popup-status">Reading the selection…</p>
      ) : null}

      {state.phase === "translating" ? (
        <p className="popup-status">Translating…</p>
      ) : null}

      {answer ? (
        <div className="popup-answer">
          <p className="popup-result">{answer.text}</p>
          <button className="popup-copy" onClick={() => void copy()}>
            {copied ? "Copied" : "Copy"}
          </button>
        </div>
      ) : null}

      {state.phase === "replacing" ? (
        <p className="popup-status">Replacing…</p>
      ) : null}

      {state.phase === "replace_failed" ? (
        <div className={`popup-failure ${state.kind}`}>
          <strong>{state.title}</strong>
          <span className="muted small">{state.detail}</span>
        </div>
      ) : null}

      {state.phase === "nothing_selected" ? (
        <p className="popup-status">
          Select something first, then press the Quick Translate shortcut again.
        </p>
      ) : null}

      {state.phase === "failed" ? (
        <div className={`popup-failure ${state.kind}`}>
          <strong>{state.title}</strong>
          <span className="muted small">{state.detail}</span>
        </div>
      ) : null}

      <p className="popup-hint muted small">
        {answer ? "Enter replaces · Esc dismisses" : "Esc dismisses"}
      </p>
    </main>
  );
}
