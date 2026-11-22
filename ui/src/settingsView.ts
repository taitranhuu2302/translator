/**
 * The Language Pair section's pure rules: one configured Target, the other
 * offered language as its implied partner, and the sentence that says the pair
 * works both ways. The component only draws them.
 */
import type { Language, LanguagePair } from "./ipc";
import { languageName } from "./languageLabel";

/** The partner chosen for each offered Target. */
const PARTNER_OF: Record<string, string> = { en: "vi", vi: "en" };

/** The sentence under the Target select. The pair is bidirectional: the Target
 * decides what text in the partner becomes, and text already in the Target goes
 * back to the partner. */
export function directionSentence(pair: LanguagePair): string {
  return `Translates both ways between ${languageName(pair.partner)} and ${languageName(pair.target)}.`;
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
