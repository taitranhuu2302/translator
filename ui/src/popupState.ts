/**
 * The Quick Translate popup as a pure state machine.
 *
 * The panel is shown before any network answer exists, so the popup always has
 * a state to draw: opening, translating, translated, nothing selected, a typed
 * failure, or dismissed. Keeping the transitions here means the rules are tested
 * without a window, a provider or a running app.
 */
import {
  formatCaptureFailure,
  formatFailure,
  type PopupFailure,
} from "./failures";
import type { EngineError, PopupPrefill, Translation } from "./ipc";

export type { FailureKind, PopupFailure } from "./failures";
export {
  formatCaptureFailure,
  formatFailure,
  providerLabel,
} from "./failures";

export type PopupState =
  | { phase: "opening" }
  | { phase: "translating"; source: string }
  | { phase: "translated"; source: string; translation: Translation }
  | { phase: "replacing"; source: string; translation: Translation }
  | ({
      phase: "replace_failed";
      source: string;
      translation: Translation;
    } & PopupFailure)
  | { phase: "nothing_selected" }
  | ({ phase: "failed" } & PopupFailure)
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
        return { phase: "nothing_selected" };
      }
      if ("failure" in prefill) {
        return failedWith(formatCaptureFailure(prefill.failure));
      }
      const source = prefill.selection.text;
      if (source.trim().length === 0) {
        return { phase: "nothing_selected" };
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
      return failedWith(formatFailure(event.error));

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
        ...formatFailure(event.error),
      };
  }
}

function failedWith(failure: PopupFailure): PopupState {
  return { phase: "failed", ...failure };
}
