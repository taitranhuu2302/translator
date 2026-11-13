import { describe, expect, it } from "vitest";

import type { HistoryEntry } from "./ipc";
import {
  filterHistory,
  historyModeLabel,
  relativeTime,
} from "./historyView";

function entry(
  source: string,
  result: string,
  overrides: Partial<HistoryEntry> = {},
): HistoryEntry {
  return {
    source,
    result,
    provider: "my_memory",
    mode: "translate",
    created_at: 1_700_000_000,
    direction: null,
    corrected: null,
    ...overrides,
  };
}

const entries = [
  entry("Xin chào thế giới", "Hello world", { created_at: 300 }),
  entry("Good morning", "Chào buổi sáng", { created_at: 200 }),
  entry("He go home.", "He's heading home.", {
    created_at: 100,
    mode: "improve",
    provider: "groq",
    corrected: "He is going home.",
  }),
];

describe("filtering history locally", () => {
  it("returns every entry for an empty query", () => {
    expect(filterHistory(entries, "")).toEqual(entries);
  });

  it("treats a whitespace-only query as empty", () => {
    expect(filterHistory(entries, "   \n\t")).toEqual(entries);
  });

  it("matches the source side case-insensitively", () => {
    expect(filterHistory(entries, "XIN CHÀO")).toEqual([entries[0]]);
  });

  it("matches the result side case-insensitively", () => {
    expect(filterHistory(entries, "chào BUỔI")).toEqual([entries[1]]);
  });

  it("matches either side of an improvement", () => {
    expect(filterHistory(entries, "heading")).toEqual([entries[2]]);
    expect(filterHistory(entries, "go home")).toEqual([entries[2]]);
  });

  it("ignores spaces around a query", () => {
    expect(filterHistory(entries, "  hello world  ")).toEqual([entries[0]]);
  });

  it("keeps a query's internal spaces literal", () => {
    expect(filterHistory(entries, "xin chào")).toEqual([entries[0]]);
    expect(filterHistory(entries, "xin  chào")).toEqual([]);
  });

  it("returns an empty list when nothing matches", () => {
    expect(filterHistory(entries, "nothing like this exists")).toEqual([]);
  });

  it("keeps the store's newest-first order", () => {
    expect(filterHistory(entries, "").map((item) => item.created_at)).toEqual([
      300, 200, 100,
    ]);
  });
});

describe("history labels", () => {
  it("names each mode", () => {
    expect(historyModeLabel("translate")).toBe("Translate");
    expect(historyModeLabel("improve")).toBe("Improve");
  });

  it("describes an entry's age in words", () => {
    const now = 1_000_000;

    expect(relativeTime(now, now)).toBe("just now");
    expect(relativeTime(now - 59, now)).toBe("just now");
    expect(relativeTime(now - 60, now)).toBe("1 minute ago");
    expect(relativeTime(now - 5 * 60, now)).toBe("5 minutes ago");
    expect(relativeTime(now - 60 * 60, now)).toBe("1 hour ago");
    expect(relativeTime(now - 3 * 60 * 60, now)).toBe("3 hours ago");
    expect(relativeTime(now - 24 * 60 * 60, now)).toBe("1 day ago");
    expect(relativeTime(now - 3 * 24 * 60 * 60, now)).toBe("3 days ago");
  });

  it("never says an entry is from the future", () => {
    expect(relativeTime(1_000_100, 1_000_000)).toBe("just now");
  });
});
