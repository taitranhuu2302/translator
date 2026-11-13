import { describe, expect, it } from "vitest";

import type { LanguagePair, Translation } from "./ipc";
import {
  EMPTY_RESULT,
  reduceResult,
  swapDirection,
  type TranslateResult,
} from "./translateWindow";

const pair: LanguagePair = { target: "en", partner: "vi" };

const translation: Translation = {
  text: "Hello",
  provider: "my_memory",
  detected_source: null,
  target: "en",
};

describe("swapping a Translate Window translation", () => {
  it("sends the result back to the language it came from", () => {
    expect(swapDirection(pair, translation)).toEqual({
      source: "en",
      target: "vi",
    });
  });

  it("uses the provider's detection when the result came from the partner", () => {
    const englishToVietnamese: Translation = {
      ...translation,
      text: "Xin chào",
      target: "vi",
      detected_source: "en",
    };

    expect(swapDirection(pair, englishToVietnamese)).toEqual({
      source: "vi",
      target: "en",
    });
  });

  it("never swaps a language into itself", () => {
    const nonsenseDetection: Translation = {
      ...translation,
      target: "en",
      detected_source: "en",
    };

    expect(swapDirection(pair, nonsenseDetection)).toEqual({
      source: "en",
      target: "vi",
    });
  });

  it("stays inside the saved pair when the detection names a third language", () => {
    const thirdLanguage: Translation = {
      ...translation,
      target: "vi",
      detected_source: "fr",
    };

    expect(swapDirection(pair, thirdLanguage)).toEqual({
      source: "vi",
      target: "en",
    });
  });

  it("reverses the direction when the saved target is the partner", () => {
    const vietnameseTargetPair: LanguagePair = { target: "vi", partner: "en" };

    expect(swapDirection(vietnameseTargetPair, translation)).toEqual({
      source: "en",
      target: "vi",
    });
  });
});

describe("the Translate Window result pane", () => {
  const started = (): TranslateResult =>
    reduceResult(EMPTY_RESULT, { type: "started" });

  it("says it is translating while the request is in flight", () => {
    expect(started()).toEqual({ phase: "translating" });
  });

  it("fills the pane with the translation", () => {
    expect(reduceResult(started(), { type: "translated", translation })).toEqual({
      phase: "translated",
      translation,
    });
  });

  it("puts a typed failure in the pane and names the provider", () => {
    const state = reduceResult(started(), {
      type: "failed",
      error: {
        provider: {
          malformed_response: { provider: "my_memory", message: "not JSON" },
        },
      },
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
    });

    expect(reduceResult(failed, { type: "started" })).toEqual({
      phase: "translating",
    });
  });

  it("clears the result when the input is emptied", () => {
    const translated = reduceResult(started(), {
      type: "translated",
      translation,
    });

    expect(reduceResult(translated, { type: "cleared" })).toEqual(EMPTY_RESULT);
  });

  it("ignores an answer that arrives without a request in flight", () => {
    expect(
      reduceResult(EMPTY_RESULT, { type: "translated", translation }),
    ).toEqual(EMPTY_RESULT);

    expect(
      reduceResult(EMPTY_RESULT, {
        type: "failed",
        error: { provider: "offline" },
      }),
    ).toEqual(EMPTY_RESULT);
  });

  it("shows a stored result when an entry is reopened, without translating", () => {
    expect(
      reduceResult(EMPTY_RESULT, { type: "restored", translation }),
    ).toEqual({ phase: "translated", translation });

    // A stored entry replaces whatever was on screen, including a failure.
    const failed = reduceResult(started(), {
      type: "failed",
      error: { provider: "offline" },
    });
    expect(
      reduceResult(failed, { type: "restored", translation }),
    ).toEqual({ phase: "translated", translation });
  });
});
