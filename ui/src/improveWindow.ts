/**
 * The Translate Window's Improve pane as logic rather than layout.
 *
 * Improve shows two labelled parts that fill in as the model writes. The
 * reducer owns the parts and the attempt's token: a chunk is appended only
 * while its token is the one in flight, and `done` replaces both parts with the
 * pair the provider actually validated — the streamed text was only progress.
 * The source text is not part of this state at all, so no answer or failure can
 * clear what the user typed.
 */
import { isCurrent, type AnswerToken } from "./answerToken";
import type { EngineError, ImprovePart, Improvement } from "./ipc";
import { noticeFor, type Notice } from "./notice";

export type ImproveResult =
  | { phase: "empty" }
  | { phase: "improving"; token: AnswerToken; corrected: string; better: string }
  | { phase: "improved"; corrected: string; better: string }
  | ({ phase: "failed" } & Notice);

export type ImproveEvent =
  | { type: "started"; token: AnswerToken }
  | { type: "chunk"; token: AnswerToken; part: ImprovePart; text: string }
  | { type: "done"; token: AnswerToken; improvement: Improvement }
  | { type: "restored"; improvement: Improvement }
  | { type: "failed"; token: AnswerToken; error: EngineError }
  | { type: "cleared" };

export const EMPTY_IMPROVEMENT: ImproveResult = { phase: "empty" };

type InFlight = Extract<ImproveResult, { phase: "improving" }>;

function isInFlight(state: ImproveResult, token: AnswerToken): state is InFlight {
  return state.phase === "improving" && isCurrent(token, state.token);
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
        token: event.token,
        corrected: "",
        better: "",
      };

    case "cleared":
      return EMPTY_IMPROVEMENT;

    // Only the attempt in flight may append. A late chunk from an abandoned
    // attempt must not leak into the pane the user is reading now.
    case "chunk": {
      if (!isInFlight(state, event.token)) return state;
      if (event.part === "corrected") {
        return { ...state, corrected: state.corrected + event.text };
      }
      return { ...state, better: state.better + event.text };
    }

    // The final pair is the authority: replace both parts even when the
    // streamed text had to be corrected on the way, and even when a part never
    // streamed at all.
    case "done": {
      if (!isInFlight(state, event.token)) return state;
      return {
        phase: "improved",
        corrected: event.improvement.corrected,
        better: event.improvement.better,
      };
    }

    // A reopened history entry fills both sections from any phase: the stored
    // pair is already the answer, so there is no "improving" step to fake and
    // no token to check.
    case "restored":
      return {
        phase: "improved",
        corrected: event.improvement.corrected,
        better: event.improvement.better,
      };

    // A failure replaces the pane with words, and only the pane.
    case "failed": {
      if (!isInFlight(state, event.token)) return state;
      return {
        phase: "failed",
        ...noticeFor(event.error),
      };
    }
  }
}

/** What each part is called on screen. The parts are never inferred from prose:
 * the provider names them, and these are the only two names. */
export function partLabel(part: ImprovePart): string {
  return part === "corrected" ? "Corrected" : "Better";
}
