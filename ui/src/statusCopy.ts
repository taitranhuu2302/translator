import type { ReplacePhase } from "./ipc";

/**
 * What a progress line says while a Replace runs.
 *
 * The popup draws the same three lines for its own phases — opening,
 * translating, replacing — so the words live here rather than in either
 * surface. The engine reports the steps; this module names them.
 */
export const REPLACE_PHASE_COPY: Record<ReplacePhase, string> = {
  capturing: "Reading the selection…",
  translating: "Translating…",
  replacing: "Replacing…",
};
