/**
 * The Translate Window as logic rather than layout.
 *
 * The result pane is a small state machine — empty, translating, translated, or
 * a typed failure — and Swap is one pure direction rule. Keeping both here means
 * the window's rules are tested without a web view, a provider or a running app,
 * and the React component only draws state and calls the IPC seam.
 */
import { isCurrent, type AnswerToken } from "./answerToken";
import type { Direction, EngineError, LanguagePair, Translation } from "./ipc";
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
 * The result text is already in `translation.target`, so reversing it means
 * asking for that language as the source. Its destination is the language the
 * text came from: the provider's detection when it named a member of the saved
 * pair, otherwise the pair's other member. Only the saved pair is read — Swap
 * never writes settings — and the rule never produces a language translated
 * into itself.
 */
export function swapDirection(
  pair: LanguagePair,
  translation: Translation,
): Direction {
  const source = translation.target;
  const other = pair.target === source ? pair.partner : pair.target;
  const detected = translation.detected_source;
  const target =
    detected !== null &&
    detected !== source &&
    (detected === pair.target || detected === pair.partner)
      ? detected
      : other;
  return { source, target };
}
