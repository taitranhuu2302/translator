import { describe, expect, it } from "vitest";

import type { LanguagePair } from "./ipc";
import { directionSentence, languageName, withTarget } from "./settingsView";

const pair = (target: string, partner: string): LanguagePair => ({
  target,
  partner,
});

describe("the Language Pair sentence", () => {
  it("says the direction the saved pair translates in", () => {
    expect(directionSentence(pair("en", "vi"))).toBe(
      "Translates from Vietnamese into English.",
    );
  });

  it("turns around when the Target changes", () => {
    expect(directionSentence(pair("vi", "en"))).toBe(
      "Translates from English into Vietnamese.",
    );
  });

  it("names a language it does not offer by its own code", () => {
    expect(languageName("ja")).toBe("ja");
    expect(directionSentence(pair("en", "ja"))).toBe(
      "Translates from ja into English.",
    );
  });
});

describe("choosing a Target", () => {
  it("makes the other offered language the partner", () => {
    expect(withTarget(pair("en", "vi"), "vi")).toEqual(pair("vi", "en"));
    expect(withTarget(pair("vi", "en"), "en")).toEqual(pair("en", "vi"));
  });

  it("leaves the pair alone when the same Target is chosen again", () => {
    const custom = pair("en", "ja");

    expect(withTarget(custom, "en")).toBe(custom);
  });
});
