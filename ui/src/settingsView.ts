/**
 * The Language Pair section's pure rules: one configured Target, the other
 * offered language as its implied partner, and the sentence that says which way
 * a translation will go. The component only draws them.
 */
import type { Language, LanguagePair } from "./ipc";

/** The two languages v1 offers. `Language` stays an open string in the model, so
 * a pair saved from outside still renders — by its own code. */
export const OFFERED_LANGUAGES = [
  { code: "en", name: "English" },
  { code: "vi", name: "Vietnamese" },
] as const;

/** The partner chosen for each offered Target. */
const PARTNER_OF: Record<string, string> = { en: "vi", vi: "en" };

export function languageName(code: Language): string {
  return (
    OFFERED_LANGUAGES.find((language) => language.code === code)?.name ?? code
  );
}

/** The sentence under the Target select. */
export function directionSentence(pair: LanguagePair): string {
  return `Translates from ${languageName(pair.partner)} into ${languageName(pair.target)}.`;
}

/**
 * Choose the Target. The partner is the other offered language, because v1 has
 * exactly two; choosing the Target already saved returns the pair untouched so a
 * partner saved elsewhere is not quietly rewritten.
 */
export function withTarget(pair: LanguagePair, target: Language): LanguagePair {
  if (pair.target === target) return pair;
  return { target, partner: PARTNER_OF[target] ?? pair.partner };
}
