/**
 * The Translate Window's Improve pane as logic rather than layout.
 *
 * Improve shows two labelled parts that fill in as the model writes. The
 * reducer owns the parts and the request id: a chunk is appended only while its
 * request is the one in flight, and `done` replaces both parts with the pair the
 * provider actually validated — the streamed text was only progress. The source
 * text is not part of this state at all, so no answer or failure can clear what
 * the user typed.
 */
import { formatFailure, type PopupFailure } from "./failures";
import type { EngineError, ImprovePart, Improvement } from "./ipc";

export type ImproveResult =
  | { phase: "empty" }
  | { phase: "improving"; requestId: number; corrected: string; better: string }
  | { phase: "improved"; requestId: number; corrected: string; better: string }
  | ({ phase: "failed"; requestId: number } & PopupFailure);

export type ImproveEvent =
  | { type: "started"; requestId: number }
  | { type: "chunk"; requestId: number; part: ImprovePart; text: string }
  | { type: "done"; requestId: number; improvement: Improvement }
  | { type: "restored"; requestId: number; improvement: Improvement }
  | { type: "failed"; requestId: number; error: EngineError }
  | { type: "cleared" };

export const EMPTY_IMPROVEMENT: ImproveResult = { phase: "empty" };

type InFlight = Extract<ImproveResult, { phase: "improving" }>;

function isInFlight(
  state: ImproveResult,
  requestId: number,
): state is InFlight {
  return state.phase === "improving" && state.requestId === requestId;
}

export function reduceImprovement(
  state: ImproveResult,
  event: ImproveEvent,
): ImproveResult {
  switch (event.type) {
    // A new Improve always starts with two empty sections, including a retry.
    case "started":
      return {
        phase: "improving",
        requestId: event.requestId,
        corrected: "",
        better: "",
      };

    case "cleared":
      return EMPTY_IMPROVEMENT;

    // Only the request in flight may append. A late chunk from an abandoned
    // attempt must not leak into the pane the user is reading now.
    case "chunk": {
      if (!isInFlight(state, event.requestId)) return state;
      if (event.part === "corrected") {
        return { ...state, corrected: state.corrected + event.text };
      }
      return { ...state, better: state.better + event.text };
    }

    // The final pair is the authority: replace both parts even when the
    // streamed text had to be corrected on the way, and even when a part never
    // streamed at all.
    case "done": {
      if (!isInFlight(state, event.requestId)) return state;
      return {
        phase: "improved",
        requestId: event.requestId,
        corrected: event.improvement.corrected,
        better: event.improvement.better,
      };
    }

    // A reopened history entry fills both sections from any phase: the stored
    // pair is already the answer, so there is no "improving" step to fake.
    case "restored":
      return {
        phase: "improved",
        requestId: event.requestId,
        corrected: event.improvement.corrected,
        better: event.improvement.better,
      };

    // A failure replaces the pane with words, and only the pane.
    case "failed": {
      if (!isInFlight(state, event.requestId)) return state;
      return {
        phase: "failed",
        requestId: event.requestId,
        ...formatFailure(event.error),
      };
    }
  }
}

/** What each part is called on screen. The parts are never inferred from prose:
 * the provider names them, and these are the only two names. */
export function partLabel(part: ImprovePart): string {
  return part === "corrected" ? "Corrected" : "Better";
}
