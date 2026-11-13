import { describe, expect, it } from "vitest";

import type { Improvement } from "./ipc";
import {
  EMPTY_IMPROVEMENT,
  partLabel,
  reduceImprovement,
  type ImproveResult,
} from "./improveWindow";

const improvement: Improvement = {
  corrected: "He goes home.",
  better: "He's heading home.",
};

function started(requestId = 1): ImproveResult {
  return reduceImprovement(EMPTY_IMPROVEMENT, { type: "started", requestId });
}

function parts(state: ImproveResult): [string, string] {
  if (state.phase !== "improving" && state.phase !== "improved") {
    throw new Error(`no parts in phase ${state.phase}`);
  }
  return [state.corrected, state.better];
}

describe("the Improve pane", () => {
  it("starts with two empty labelled parts", () => {
    expect(started()).toEqual({
      phase: "improving",
      requestId: 1,
      corrected: "",
      better: "",
    });
  });

  it("appends each chunk to the part it names", () => {
    let state = started();
    state = reduceImprovement(state, {
      type: "chunk",
      requestId: 1,
      part: "better",
      text: "He's ",
    });
    state = reduceImprovement(state, {
      type: "chunk",
      requestId: 1,
      part: "corrected",
      text: "He goes ",
    });
    state = reduceImprovement(state, {
      type: "chunk",
      requestId: 1,
      part: "better",
      text: "heading home.",
    });
    state = reduceImprovement(state, {
      type: "chunk",
      requestId: 1,
      part: "corrected",
      text: "home.",
    });

    expect(parts(state)).toEqual(["He goes home.", "He's heading home."]);
  });

  it("replaces both parts with the final pair, never merging or inferring", () => {
    // The stream stalled half-way; done still has to show the whole answer.
    const stalled = reduceImprovement(started(), {
      type: "chunk",
      requestId: 1,
      part: "corrected",
      text: "He go",
    });

    expect(
      reduceImprovement(stalled, { type: "done", requestId: 1, improvement }),
    ).toEqual({
      phase: "improved",
      requestId: 1,
      corrected: "He goes home.",
      better: "He's heading home.",
    });
  });

  it("ignores a chunk from a request that is no longer in flight", () => {
    const state = started(2);

    expect(
      reduceImprovement(state, {
        type: "chunk",
        requestId: 1,
        part: "corrected",
        text: "late",
      }),
    ).toBe(state);
  });

  it("ignores a done or failed answer from an abandoned request", () => {
    const state = started(2);

    expect(
      reduceImprovement(state, { type: "done", requestId: 1, improvement }),
    ).toBe(state);
    expect(
      reduceImprovement(state, {
        type: "failed",
        requestId: 1,
        error: { provider: "offline" },
      }),
    ).toBe(state);
  });

  it("ignores an answer once the request has finished", () => {
    const finished = reduceImprovement(started(), {
      type: "done",
      requestId: 1,
      improvement,
    });

    expect(
      reduceImprovement(finished, {
        type: "chunk",
        requestId: 1,
        part: "better",
        text: "late",
      }),
    ).toBe(finished);
  });

  it("shows a typed failure and keeps whatever already streamed out of the way", () => {
    const state = reduceImprovement(started(), {
      type: "failed",
      requestId: 1,
      error: {
        provider: { missing_key: { provider: "groq" } },
      },
    });

    expect(state.phase).toBe("failed");
    if (state.phase !== "failed") return;
    expect(state.title).toContain("Groq");
    expect(state.title.toLowerCase()).toContain("api key");
    expect(state.detail).toContain("Groq");
  });

  it("names the missing key instead of saying no provider is available", () => {
    const state = reduceImprovement(started(), {
      type: "failed",
      requestId: 1,
      error: { provider: { missing_key: { provider: "groq" } } },
    });

    if (state.phase !== "failed") throw new Error("expected a failure");
    expect(state.title.toLowerCase()).not.toContain("no translation provider");
    expect(state.title).toContain("Groq needs an API key");
  });

  it("clears both parts when the input is emptied", () => {
    const improved = reduceImprovement(started(), {
      type: "done",
      requestId: 1,
      improvement,
    });

    expect(reduceImprovement(improved, { type: "cleared" })).toEqual(
      EMPTY_IMPROVEMENT,
    );
  });

  it("has only two part names", () => {
    expect(partLabel("corrected")).toBe("Corrected");
    expect(partLabel("better")).toBe("Better");
  });
});
