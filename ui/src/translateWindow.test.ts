import { describe, expect, it } from "vitest";

import type { AnswerToken } from "./answerToken";
import type { Translation } from "./ipc";
import {
  effectiveDirection,
  EMPTY_RESULT,
  reduceResult,
  swapDirection,
  type TranslateResult,
} from "./translateWindow";

const translation: Translation = {
  text: "Hello",
  provider: "my_memory",
  source: "vi",
  detected_source: null,
  target: "en",
};

describe("swapping a Translate Window translation", () => {
  it("sends the result back to the language it came from", () => {
    expect(swapDirection(translation)).toEqual({
      source: "en",
      target: "vi",
    });
  });

  it("reverses a translation that went into Vietnamese", () => {
    expect(
      swapDirection({
        text: "Xin chào",
        provider: "azure",
        source: "en",
        detected_source: "en",
        target: "vi",
      }),
    ).toEqual({
      source: "vi",
      target: "en",
    });
  });

  it("ignores whatever the provider reported detecting", () => {
    expect(swapDirection({ ...translation, detected_source: "fr" })).toEqual({
      source: "en",
      target: "vi",
    });
  });
});

describe("the direction the Source pane names", () => {
  const detected = { source: "vi", target: "en" };
  const chosen = { source: "en", target: "vi" };

  it("shows the live Detection while nothing has been chosen", () => {
    expect(effectiveDirection(null, detected)).toEqual(detected);
  });

  it("shows a chosen direction over the Detection, because that is what submits", () => {
    expect(effectiveDirection(chosen, detected)).toEqual(chosen);
  });

  it("has nothing to name when neither exists", () => {
    expect(effectiveDirection(null, null)).toBeNull();
  });
});

describe("the Translate Window result pane", () => {
  const token: AnswerToken = { session: 1, sequence: 1 };

  const started = (at: AnswerToken = token): TranslateResult =>
    reduceResult(EMPTY_RESULT, { type: "started", token: at });

  it("says it is translating while the request is in flight", () => {
    expect(started()).toEqual({ phase: "translating", token });
  });

  it("fills the pane with the translation", () => {
    expect(
      reduceResult(started(), { type: "translated", translation, token }),
    ).toEqual({ phase: "translated", translation });
  });

  it("puts a typed failure in the pane and names the provider", () => {
    const state = reduceResult(started(), {
      type: "failed",
      error: {
        provider: {
          malformed_response: { provider: "my_memory", message: "not JSON" },
        },
      },
      token,
    });

    expect(state.phase).toBe("failed");
    if (state.phase !== "failed") return;
    expect(state.kind).toBe("provider");
    expect(state.detail).toContain("MyMemory");
  });

  it("calls an offline failure offline instead of a generic provider failure", () => {
    const state = reduceResult(started(), {
      type: "failed",
      error: { provider: "offline" },
      token,
    });

    expect(state.phase).toBe("failed");
    if (state.phase !== "failed") return;
    expect(state.kind).toBe("offline");
    expect(state.title.toLowerCase()).toContain("offline");
  });

  it("can be retried after a failure without losing the pane", () => {
    const failed = reduceResult(started(), {
      type: "failed",
      error: { provider: "offline" },
      token,
    });
    const retry = { session: 1, sequence: 2 };

    expect(reduceResult(failed, { type: "started", token: retry })).toEqual({
      phase: "translating",
      token: retry,
    });
  });

  it("clears the result when the input is emptied", () => {
    const translated = reduceResult(started(), {
      type: "translated",
      translation,
      token,
    });

    expect(reduceResult(translated, { type: "cleared" })).toEqual(EMPTY_RESULT);
  });

  it("ignores an answer that arrives without a request in flight", () => {
    expect(
      reduceResult(EMPTY_RESULT, { type: "translated", translation, token }),
    ).toEqual(EMPTY_RESULT);

    expect(
      reduceResult(EMPTY_RESULT, {
        type: "failed",
        error: { provider: "offline" },
        token,
      }),
    ).toEqual(EMPTY_RESULT);
  });

  it("ignores a late answer from an attempt the pane has abandoned", () => {
    const abandoned: AnswerToken = { session: 1, sequence: 1 };
    const current: AnswerToken = { session: 1, sequence: 2 };
    const state = started(current);

    expect(
      reduceResult(state, {
        type: "translated",
        translation,
        token: abandoned,
      }),
    ).toBe(state);
    expect(
      reduceResult(state, {
        type: "failed",
        error: { provider: "offline" },
        token: abandoned,
      }),
    ).toBe(state);
  });

  it("shows a stored result when an entry is reopened, without translating", () => {
    // A restored entry answers no request, so it needs no token and applies
    // from any phase — even one nobody is waiting on.
    expect(
      reduceResult(EMPTY_RESULT, { type: "restored", translation }),
    ).toEqual({ phase: "translated", translation });

    // A stored entry replaces whatever was on screen, including a failure.
    const failed = reduceResult(started(), {
      type: "failed",
      error: { provider: "offline" },
      token,
    });
    expect(
      reduceResult(failed, { type: "restored", translation }),
    ).toEqual({ phase: "translated", translation });
  });
});
