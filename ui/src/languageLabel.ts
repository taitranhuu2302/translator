/**
 * How a language and a Direction are written in the interface.
 *
 * One module owns the offered languages and the arrow spelling, so the
 * Language Pair sentence, the Translate Window and the popup cannot drift.
 */
import type { Direction, Language } from "./ipc";

/** The two languages v1 offers. `Language` stays an open string in the model, so
 * a pair saved from outside still renders — by its own code. */
export const OFFERED_LANGUAGES = [
  { code: "en", name: "English" },
  { code: "vi", name: "Vietnamese" },
] as const;

export function languageName(code: Language): string {
  return (
    OFFERED_LANGUAGES.find((language) => language.code === code)?.name ?? code
  );
}

/** "Vietnamese → English". Full names: a result is read once, and a two-letter
 * code would need translating itself. */
export function directionLabel(direction: Direction): string {
  return `${languageName(direction.source)} → ${languageName(direction.target)}`;
}
