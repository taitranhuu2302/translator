/**
 * The Quick Translate popup as a pure state machine.
 *
 * The panel is shown before any network answer exists, so the popup always has
 * a state to draw: opening, translating, translated, nothing selected, a typed
 * failure, or dismissed. Keeping the transitions here means the rules are tested
 * without a window, a provider or a running app.
 */
import type { EngineError, PopupPrefill, Translation } from "./ipc";
import { noticeFor, noticeForCapture, type Notice } from "./notice";

export type PopupState =
  | { phase: "opening" }
  | { phase: "translating"; source: string }
  | { phase: "translated"; source: string; translation: Translation }
  | { phase: "replacing"; source: string; translation: Translation }
  | ({
      phase: "replace_failed";
      source: string;
      translation: Translation;
    } & Notice)
  | ({ phase: "nothing_selected" } & Notice)
  | ({ phase: "failed" } & Notice)
  | { phase: "dismissed" };

export type PopupEvent =
  | { type: "prefill"; prefill: PopupPrefill }
  | { type: "translated"; translation: Translation }
  | { type: "failed"; error: EngineError }
  | { type: "replace_requested" }
  | { type: "replace_failed"; error: EngineError }
  | { type: "dismissed" };

export const OPENING: PopupState = { phase: "opening" };

export function reduce(state: PopupState, event: PopupEvent): PopupState {
  switch (event.type) {
    // The same panel is reused, so a prefill always starts a fresh session —
    // even after the previous answer was dismissed.
    case "prefill": {
      const prefill = event.prefill;
      if (prefill === "nothing_selected") {
        return nothingSelected();
      }
      if ("failure" in prefill) {
        return failedWith(noticeForCapture(prefill.failure));
      }
      const source = prefill.selection.text;
      // The engine now enforces the blank rule; this check only improves the
      // affordance, saying "nothing selected" instead of showing a failure.
      if (source.trim().length === 0) {
        return nothingSelected();
      }
      return { phase: "translating", source };
    }

    case "dismissed":
      return { phase: "dismissed" };

    case "translated":
      // Once dismissed, a late answer must not bring the popup back.
      if (state.phase !== "translating") {
        return state;
      }
      return { phase: "translated", source: state.source, translation: event.translation };

    case "failed":
      if (state.phase !== "translating") {
        return state;
      }
      return failedWith(noticeFor(event.error));

    // Enter commits the translation that is on screen. There is no second
    // translation and no re-read of the Selection: the user gets exactly the
    // text they are looking at.
    case "replace_requested":
      if (state.phase !== "translated") {
        return state;
      }
      return { phase: "replacing", source: state.source, translation: state.translation };

    // A refused Replace leaves the popup open with the reason inline, and the
    // same translation still available to Copy.
    case "replace_failed":
      if (state.phase !== "replacing") {
        return state;
      }
      return {
        phase: "replace_failed",
        source: state.source,
        translation: state.translation,
        ...noticeFor(event.error),
      };
  }
}

function failedWith(notice: Notice): PopupState {
  return { phase: "failed", ...notice };
}

/** Both ways of arriving with no text — an empty prefill and a trimmed-to-empty
 * selection — are the same outcome and the same words. */
function nothingSelected(): PopupState {
  return { phase: "nothing_selected", ...noticeForCapture("nothing_selected") };
}
