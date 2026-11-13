import { describe, expect, it } from "vitest";

import type { Translation } from "./ipc";
import { OPENING, formatCaptureFailure, formatFailure, reduce } from "./popupState";

const translation: Translation = {
  text: "Hello world",
  provider: "my_memory",
  detected_source: "vi",
  target: "en",
};

describe("the Quick Translate popup", () => {
  it("shows the translating state before any answer exists", () => {
    const state = reduce(OPENING, {
      type: "prefill",
      prefill: { selection: { text: "Xin chào thế giới" } },
    });

    expect(state).toEqual({ phase: "translating", source: "Xin chào thế giới" });
  });

  it("replaces the translating state with the translation in the same popup", () => {
    const translating = reduce(OPENING, {
      type: "prefill",
      prefill: { selection: { text: "Xin chào thế giới" } },
    });

    const translated = reduce(translating, { type: "translated", translation });

    expect(translated).toEqual({
      phase: "translated",
      source: "Xin chào thế giới",
      translation,
    });
  });

  it("says nothing was selected instead of showing an empty translation", () => {
    expect(reduce(OPENING, { type: "prefill", prefill: "nothing_selected" })).toEqual({
      phase: "nothing_selected",
    });

    expect(
      reduce(OPENING, { type: "prefill", prefill: { selection: { text: "  \n " } } }),
    ).toEqual({ phase: "nothing_selected" });
  });

  it("names an offline failure as offline, not as a generic error", () => {
    const translating = reduce(OPENING, {
      type: "prefill",
      prefill: { selection: { text: "Xin chào" } },
    });

    const state = reduce(translating, {
      type: "failed",
      error: { provider: "offline" },
    });

    expect(state.phase).toBe("failed");
    if (state.phase !== "failed") return;
    expect(state.kind).toBe("offline");
    expect(state.title.toLowerCase()).toContain("offline");
    expect(state.detail.toLowerCase()).toContain("network");
  });

  it("keeps another provider failure a provider failure, with the provider named", () => {
    const failure = formatFailure({
      provider: { malformed_response: { provider: "my_memory", message: "not JSON" } },
    });

    expect(failure.kind).toBe("provider");
    expect(failure.detail).toContain("MyMemory");
    expect(failure.title.toLowerCase()).not.toContain("offline");
  });

  it("explains a capture failure from the prefill", () => {
    const state = reduce(OPENING, {
      type: "prefill",
      prefill: { failure: "no_accessibility" },
    });

    expect(state.phase).toBe("failed");
    if (state.phase !== "failed") return;
    expect(state.kind).toBe("capture");
    expect(state.title).toContain("Accessibility");
  });

  it("starts over when the same panel is opened again", () => {
    const dismissed = reduce(
      reduce(OPENING, { type: "prefill", prefill: "nothing_selected" }),
      { type: "dismissed" },
    );

    expect(
      reduce(dismissed, { type: "prefill", prefill: { selection: { text: "Xin chào" } } }),
    ).toEqual({ phase: "translating", source: "Xin chào" });

    const answered = reduce(reduce(OPENING, {
      type: "prefill",
      prefill: { selection: { text: "Xin chào" } },
    }), { type: "translated", translation });

    expect(
      reduce(answered, { type: "prefill", prefill: { selection: { text: "Tạm biệt" } } }),
    ).toEqual({ phase: "translating", source: "Tạm biệt" });
  });

  it("replaces with exactly the translation on screen and does not start over", () => {
    const translated = reduce(
      reduce(OPENING, {
        type: "prefill",
        prefill: { selection: { text: "Xin chào thế giới" } },
      }),
      { type: "translated", translation },
    );

    const replacing = reduce(translated, { type: "replace_requested" });

    expect(replacing).toEqual({
      phase: "replacing",
      source: "Xin chào thế giới",
      translation,
    });
  });

  it("will not replace before the translation exists", () => {
    const translating = reduce(OPENING, {
      type: "prefill",
      prefill: { selection: { text: "Xin chào" } },
    });

    expect(reduce(translating, { type: "replace_requested" })).toEqual(translating);
  });

  it("keeps the popup open with an inline failure when the app refuses the replace", () => {
    const translated = reduce(
      reduce(OPENING, {
        type: "prefill",
        prefill: { selection: { text: "Xin chào" } },
      }),
      { type: "translated", translation },
    );
    const replacing = reduce(translated, { type: "replace_requested" });

    const failed = reduce(replacing, {
      type: "replace_failed",
      error: { capture: "not_writable" },
    });

    expect(failed.phase).toBe("replace_failed");
    if (failed.phase !== "replace_failed") return;
    expect(failed.translation).toEqual(translation);
    expect(failed.source).toBe("Xin chào");
    expect(failed.kind).toBe("capture");
    expect(failed.title.toLowerCase()).toContain("did not accept");
  });

  it("explains a refusal to replace with no captured selection", () => {
    const failure = formatCaptureFailure("nothing_to_replace");

    expect(failure.kind).toBe("capture");
    expect(failure.detail.toLowerCase()).toContain("select");
  });

  it("explains a secure field refusing the replace", () => {
    const failure = formatCaptureFailure("secure_input");

    expect(failure.title.toLowerCase()).toContain("secure");
  });

  it("stays dismissed even if a slow answer lands afterwards", () => {
    const translating = reduce(OPENING, {
      type: "prefill",
      prefill: { selection: { text: "Xin chào" } },
    });
    const dismissed = reduce(translating, { type: "dismissed" });

    expect(dismissed).toEqual({ phase: "dismissed" });
    expect(reduce(dismissed, { type: "translated", translation })).toEqual({
      phase: "dismissed",
    });
  });
});
