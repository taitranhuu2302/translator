/**
 * The Translate Window as logic rather than layout.
 *
 * The result pane is a small state machine — empty, translating, translated, or
 * a typed failure — and Swap is one pure direction rule. Keeping both here means
 * the window's rules are tested without a web view, a provider or a running app,
 * and the React component only draws state and calls the IPC seam.
 */
import { isCurrent, type AnswerToken } from "./answerToken";
import type { Direction, EngineError, Translation } from "./ipc";
import { noticeFor, type Notice } from "./notice";

export type TranslateResult =
  | { phase: "empty" }
  | { phase: "translating"; token: AnswerToken }
  | { phase: "translated"; translation: Translation }
  | ({ phase: "failed" } & Notice);

export type TranslateEvent =
  | { type: "started"; token: AnswerToken }
  | { type: "translated"; translation: Translation; token: AnswerToken }
  | { type: "restored"; translation: Translation }
  | { type: "failed"; error: EngineError; token: AnswerToken }
  | { type: "cleared" };

export const EMPTY_RESULT: TranslateResult = { phase: "empty" };

export function reduceResult(
  state: TranslateResult,
  event: TranslateEvent,
): TranslateResult {
  switch (event.type) {
    // A new translation always starts a fresh pane, including a retry after a
    // failure. The source text is the component's, never discarded here.
    case "started":
      return { phase: "translating", token: event.token };

    case "cleared":
      return EMPTY_RESULT;

    // Only the token this pane is waiting on may fill it; a late answer from
    // an abandoned attempt must not replace what the user is reading.
    case "translated":
      if (state.phase !== "translating" || !isCurrent(event.token, state.token)) {
        return state;
      }
      return { phase: "translated", translation: event.translation };

    // A reopened history entry is already an answer: it fills the pane from any
    // phase without passing through "translating", and it answers no request,
    // so it carries no token.
    case "restored":
      return { phase: "translated", translation: event.translation };

    case "failed":
      if (state.phase !== "translating" || !isCurrent(event.token, state.token)) {
        return state;
      }
      return { phase: "failed", ...noticeFor(event.error) };
  }
}

/**
 * The direction the next translation runs in after Swap.
 *
 * The answer knows the direction it was produced in, so reversing it is a
 * reversal of that value: the result's language becomes the source, and the
 * language it came from becomes the target. Nothing is guessed from the saved
 * pair or from a provider's reported detection, and Swap never writes settings.
 */
export function swapDirection(translation: Translation): Direction {
  return { source: translation.target, target: translation.source };
}

/**
 * The direction the next Translate would run in, as the Source pane names it.
 *
 * A direction the window has explicitly chosen — Swap, or a reopened history
 * entry — wins over the live Detection, because that is what submitting will
 * actually use. Editing the text clears the choice, and the Detection shows
 * again.
 */
export function effectiveDirection(
  chosen: Direction | null,
  detected: Direction | null,
): Direction | null {
  return chosen ?? detected;
}
