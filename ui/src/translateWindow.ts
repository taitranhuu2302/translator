/**
 * The Translate Window as logic rather than layout.
 *
 * The result pane is a small state machine — empty, translating, translated, or
 * a typed failure — and Swap is one pure direction rule. Keeping both here means
 * the window's rules are tested without a web view, a provider or a running app,
 * and the React component only draws state and calls the IPC seam.
 */
import { formatFailure } from "./failures";
import type { Direction, EngineError, LanguagePair, Translation } from "./ipc";

export type TranslateResult =
  | { phase: "empty" }
  | { phase: "translating" }
  | { phase: "translated"; translation: Translation }
  | ({ phase: "failed" } & ReturnType<typeof formatFailure>);

export type TranslateEvent =
  | { type: "started" }
  | { type: "translated"; translation: Translation }
  | { type: "restored"; translation: Translation }
  | { type: "failed"; error: EngineError }
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
      return { phase: "translating" };

    case "cleared":
      return EMPTY_RESULT;

    // Only an answer to a request in flight may fill the pane; a late answer
    // from an abandoned attempt must not replace what the user is reading.
    case "translated":
      if (state.phase !== "translating") return state;
      return { phase: "translated", translation: event.translation };

    // A reopened history entry is already an answer: it fills the pane from any
    // phase without passing through "translating", and the reducer never emits
    // a request on its behalf.
    case "restored":
      return { phase: "translated", translation: event.translation };

    case "failed":
      if (state.phase !== "translating") return state;
      return { phase: "failed", ...formatFailure(event.error) };
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
