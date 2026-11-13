/**
 * The History surface's rules as logic rather than layout.
 *
 * The shell hands over recent entries once and the field filters them locally as
 * the user types: no IPC per keystroke and no network anywhere. Matching works
 * on either side of the entry, case-insensitively, so a Vietnamese query finds
 * the English translation and the reverse.
 */
import type { HistoryEntry, Mode } from "./ipc";

/** Keep the entries whose source or result contains `query`, ignoring case and
 * surrounding spaces. An empty query keeps everything. */
export function filterHistory(
  entries: HistoryEntry[],
  query: string,
): HistoryEntry[] {
  const needle = query.trim().toLowerCase();
  if (needle.length === 0) return entries;
  return entries.filter(
    (entry) =>
      entry.source.toLowerCase().includes(needle) ||
      entry.result.toLowerCase().includes(needle),
  );
}

/** What each entry's mode is called on screen. */
export function historyModeLabel(mode: Mode): string {
  return mode === "improve" ? "Improve" : "Translate";
}

/** When an entry happened, in words. `nowSeconds` is passed in so the rule is
 * testable without a clock; a timestamp from the future reads as "just now". */
export function relativeTime(createdAt: number, nowSeconds: number): string {
  const elapsed = Math.max(0, Math.floor(nowSeconds - createdAt));
  if (elapsed < 60) return "just now";

  const minutes = Math.floor(elapsed / 60);
  if (minutes < 60) return plural(minutes, "minute");

  const hours = Math.floor(minutes / 60);
  if (hours < 24) return plural(hours, "hour");

  return plural(Math.floor(hours / 24), "day");
}

function plural(count: number, unit: string): string {
  return `${count} ${unit}${count === 1 ? "" : "s"} ago`;
}
