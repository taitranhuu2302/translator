import { describe, expect, it } from "vitest";

import { directionLabel, languageName } from "./languageLabel";

describe("writing a Direction", () => {
  it("names both languages in full, source first", () => {
    expect(directionLabel({ source: "vi", target: "en" })).toBe(
      "Vietnamese → English",
    );
  });

  it("turns around with the direction", () => {
    expect(directionLabel({ source: "en", target: "vi" })).toBe(
      "English → Vietnamese",
    );
  });

  it("writes a language it does not offer by its own code", () => {
    expect(directionLabel({ source: "ja", target: "en" })).toBe("ja → English");
    expect(languageName("ja")).toBe("ja");
  });
});
