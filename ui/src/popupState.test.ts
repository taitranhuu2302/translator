import { describe, expect, it } from "vitest";

import type { AnswerToken } from "./answerToken";
import type { Translation } from "./ipc";
import { noticeFor, noticeForCapture } from "./notice";
import { OPENING, reduce } from "./popupState";

const translation: Translation = {
  text: "Hello world",
  provider: "my_memory",
  source: "vi",
  detected_source: "vi",
  target: "en",
};

/** The token one opening of the panel issues for its translation. */
const token: AnswerToken = { session: 1, sequence: 1 };

function opened(text: string, at: AnswerToken = token) {
  return reduce(OPENING, {
    type: "prefill",
    prefill: { selection: { text } },
    token: at,
  });
}

describe("the Quick Translate popup", () => {
  it("shows the translating state before any answer exists", () => {
    expect(opened("Xin chào thế giới")).toEqual({
      phase: "translating",
      source: "Xin chào thế giới",
      token,
    });
  });

  it("replaces the translating state with the translation in the same popup", () => {
    const translated = reduce(opened("Xin chào thế giới"), {
      type: "translated",
      translation,
      token,
    });

    expect(translated).toEqual({
      phase: "translated",
      source: "Xin chào thế giới",
      translation,
    });
  });

  it("says nothing was selected instead of showing an empty translation", () => {
    const nothingSelected = {
      phase: "nothing_selected",
      ...noticeForCapture("nothing_selected"),
    };

    expect(
      reduce(OPENING, {
        type: "prefill",
        prefill: "nothing_selected",
        token,
      }),
    ).toEqual(nothingSelected);

    expect(
      reduce(OPENING, {
        type: "prefill",
        prefill: { selection: { text: "  \n " } },
        token,
      }),
    ).toEqual(nothingSelected);
  });

  it("names an offline failure as offline, not as a generic error", () => {
    const state = reduce(opened("Xin chào"), {
      type: "failed",
      error: { provider: "offline" },
      token,
    });

    expect(state.phase).toBe("failed");
    if (state.phase !== "failed") return;
    expect(state.kind).toBe("offline");
    expect(state.title.toLowerCase()).toContain("offline");
    expect(state.detail.toLowerCase()).toContain("network");
  });

  it("keeps another provider failure a provider failure, with the provider named", () => {
    const failure = noticeFor({
      provider: { malformed_response: { provider: "my_memory", message: "not JSON" } },
    });

    expect(failure.kind).toBe("provider");
    expect(failure.detail).toContain("MyMemory");
    expect(failure.title.toLowerCase()).not.toContain("offline");
  });

  it("a bad key names the provider and points at Settings", () => {
    const failure = noticeFor({
      provider: {
        authentication_failed: { provider: "azure", cause: "invalid_key" },
      },
    });

    expect(failure.kind).toBe("provider");
    expect(failure.title.toLowerCase()).toContain("key");
    expect(failure.detail).toContain("Azure");
    expect(failure.detail).toContain("new key in Settings");
  });

  it("an authentication failure written before the cause points at the key", () => {
    const failure = noticeFor({
      provider: { authentication_failed: { provider: "azure" } },
    });

    expect(failure.detail).toContain("new key in Settings");
  });

  it("a refused Azure resource points at the resource, never at billing quota", () => {
    const failure = noticeFor({
      provider: {
        authentication_failed: {
          provider: "azure",
          cause: "resource_not_authorized",
        },
      },
    });

    expect(failure.kind).toBe("provider");
    expect(failure.title).toContain("Azure");
    expect(failure.detail.toLowerCase()).toContain("resource");
    expect(failure.detail.toLowerCase()).not.toContain("quota");
    expect(failure.detail.toLowerCase()).not.toContain("rejected its key");
  });

  it("explains a capture failure from the prefill", () => {
    const state = reduce(OPENING, {
      type: "prefill",
      prefill: { failure: "no_accessibility" },
      token,
    });

    expect(state.phase).toBe("failed");
    if (state.phase !== "failed") return;
    expect(state.kind).toBe("capture");
    expect(state.title).toContain("Accessibility");
  });

  it("starts over when the same panel is opened again", () => {
    const next = { session: 2, sequence: 1 };
    const dismissed = reduce(
      reduce(OPENING, {
        type: "prefill",
        prefill: "nothing_selected",
        token,
      }),
      { type: "dismissed" },
    );

    expect(
      reduce(dismissed, {
        type: "prefill",
        prefill: { selection: { text: "Xin chào" } },
        token: next,
      }),
    ).toEqual({ phase: "translating", source: "Xin chào", token: next });

    const answered = reduce(opened("Xin chào"), {
      type: "translated",
      translation,
      token,
    });

    expect(
      reduce(answered, {
        type: "prefill",
        prefill: { selection: { text: "Tạm biệt" } },
        token: next,
      }),
    ).toEqual({ phase: "translating", source: "Tạm biệt", token: next });
  });

  it("drops a late answer from the open that was replaced while it was translating", () => {
    // Both opens are translating; only the token says which answer is theirs.
    const previous = { session: 1, sequence: 1 };
    const reopened = { session: 2, sequence: 1 };
    const state = reduce(opened("Xin chào", previous), {
      type: "prefill",
      prefill: { selection: { text: "Tạm biệt" } },
      token: reopened,
    });

    expect(
      reduce(state, { type: "translated", translation, token: previous }),
    ).toEqual({ phase: "translating", source: "Tạm biệt", token: reopened });
  });

  it("accepts the reopened session's answer and ignores the one it replaced", () => {
    const previous = { session: 1, sequence: 1 };
    const reopened = { session: 2, sequence: 1 };
    const state = reduce(opened("Xin chào", previous), {
      type: "prefill",
      prefill: { selection: { text: "Tạm biệt" } },
      token: reopened,
    });

    const translated = reduce(state, {
      type: "translated",
      translation: { ...translation, text: "Goodbye" },
      token: reopened,
    });

    expect(translated).toEqual({
      phase: "translated",
      source: "Tạm biệt",
      translation: { ...translation, text: "Goodbye" },
    });
  });

  it("drops a late failure from an open that has been replaced", () => {
    const previous = { session: 1, sequence: 1 };
    const reopened = { session: 2, sequence: 1 };
    const state = reduce(opened("Xin chào", previous), {
      type: "prefill",
      prefill: { selection: { text: "Tạm biệt" } },
      token: reopened,
    });

    expect(
      reduce(state, {
        type: "failed",
        error: { provider: "offline" },
        token: previous,
      }),
    ).toEqual({ phase: "translating", source: "Tạm biệt", token: reopened });
  });

  it("replaces with exactly the translation on screen and does not start over", () => {
    const translated = reduce(opened("Xin chào thế giới"), {
      type: "translated",
      translation,
      token,
    });

    const replacing = reduce(translated, { type: "replace_requested" });

    expect(replacing).toEqual({
      phase: "replacing",
      source: "Xin chào thế giới",
      translation,
    });
  });

  it("will not replace before the translation exists", () => {
    const translating = opened("Xin chào");

    expect(reduce(translating, { type: "replace_requested" })).toEqual(translating);
  });

  it("keeps the popup open with an inline failure when the app refuses the replace", () => {
    const translated = reduce(opened("Xin chào"), {
      type: "translated",
      translation,
      token,
    });
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
    const failure = noticeForCapture("nothing_to_replace");

    expect(failure.kind).toBe("capture");
    expect(failure.detail.toLowerCase()).toContain("select");
  });

  it("explains a secure field refusing the replace", () => {
    const failure = noticeForCapture("secure_input");

    expect(failure.title.toLowerCase()).toContain("secure");
  });

  it("stays dismissed even if a slow answer lands afterwards", () => {
    const dismissed = reduce(opened("Xin chào"), { type: "dismissed" });

    expect(dismissed).toEqual({ phase: "dismissed" });
    expect(
      reduce(dismissed, { type: "translated", translation, token }),
    ).toEqual({ phase: "dismissed" });
  });
});
