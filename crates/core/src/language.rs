use serde::{Deserialize, Serialize};

/// A language the app can translate to or from, identified by its provider code
/// (`en`, `vi`, …).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Language(String);

impl Language {
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into())
    }

    pub fn english() -> Self {
        Self::new("en")
    }

    pub fn vietnamese() -> Self {
        Self::new("vi")
    }

    pub fn code(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// One configured Target and the partner implied by it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguagePair {
    pub target: Language,
    pub partner: Language,
}

impl LanguagePair {
    pub fn new(target: Language, partner: Language) -> Self {
        Self { target, partner }
    }

    /// The language text in `detected` or unknown text is translated into:
    /// text already in the Target goes to the partner, everything else to the Target.
    pub fn destination_for(&self, detected: Option<&Language>) -> Language {
        match detected {
            Some(language) if *language == self.target => self.partner.clone(),
            _ => self.target.clone(),
        }
    }
}

impl Default for LanguagePair {
    fn default() -> Self {
        Self::new(Language::english(), Language::vietnamese())
    }
}

/// The two languages one piece of text is translated between. `source` is never
/// `target`: a provider rejects `en|en`, so a degenerate pair falls back to the
/// other language of the v1 pair instead of asking for a language into itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Direction {
    pub source: Language,
    pub target: Language,
}

impl LanguagePair {
    /// Resolve the direction for `text`. Text already in the Target goes to the
    /// partner; anything else goes to the Target. The source is the local guess
    /// when there is one, otherwise the pair's implicit input language, so every
    /// request carries a complete language pair.
    pub fn direction_for(&self, text: &str) -> Direction {
        let detected = guess_language(text);
        let source = detected
            .clone()
            .unwrap_or_else(|| self.partner.clone());
        let mut target = self.destination_for(detected.as_ref());
        if target == source {
            target = other_of_the_v1_pair(&source);
        }
        Direction { source, target }
    }
}

/// The guard's escape hatch. v1 only ever resolves English and Vietnamese, so a
/// pair that names the same language twice still has a sensible other side.
fn other_of_the_v1_pair(language: &Language) -> Language {
    if *language == Language::english() {
        Language::vietnamese()
    } else {
        Language::english()
    }
}

/// A lightweight guess at which language some text is written in, for picking a
/// speech voice. It is not the provider ladder's detection (ticket 06): it reads
/// Vietnamese letters and tone marks, calls the rest of the Latin alphabet
/// English, and admits it cannot tell when the text carries no letter it knows.
pub fn guess_language(text: &str) -> Option<Language> {
    let mut saw_latin_letter = false;
    for character in text.chars() {
        if carries_vietnamese_mark(character) {
            return Some(Language::vietnamese());
        }
        if is_latin_letter(character) {
            saw_latin_letter = true;
        }
    }
    saw_latin_letter.then(Language::english)
}

/// The letters and tone marks Vietnamese adds to the Latin alphabet. A tone-marked
/// vowel is Vietnamese evidence even where another language also uses it; the
/// heuristic only ever has to choose between Vietnamese and English.
fn carries_vietnamese_mark(character: char) -> bool {
    const VIETNAMESE_LETTERS: &str = "ăâđêôơưĂÂĐÊÔƠƯ";
    const VIETNAMESE_TONES: &str = "àáảãạầấẩẫậằắẳẵặèéẻẽẹềếểễệìíỉĩịòóỏõọồốổỗộờớởỡợùúủũụừứửữựỳýỷỹỵ\
        ÀÁẢÃẠẦẤẨẪẬẰẮẲẴẶÈÉẺẼẸỀẾỂỄỆÌÍỈĨỊÒÓỎÕỌỒỐỔỖỘỜỚỞỠỢÙÚỦŨỤỪỨỬỮỰỲÝỶỸỴ";
    /// The same tones written as combining marks, which some input methods emit.
    const COMBINING_TONES: &str = "\u{0300}\u{0301}\u{0303}\u{0309}\u{0323}";

    VIETNAMESE_LETTERS.contains(character)
        || VIETNAMESE_TONES.contains(character)
        || COMBINING_TONES.contains(character)
}

/// Latin letters, including the accented ones, but not the punctuation that
/// shares their Unicode range.
fn is_latin_letter(character: char) -> bool {
    character.is_ascii_alphabetic()
        || (('\u{00C0}'..='\u{024F}').contains(&character) && character.is_alphabetic())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_text_is_english() {
        assert_eq!(guess_language("Hello, world"), Some(Language::english()));
        assert_eq!(guess_language("Good morning"), Some(Language::english()));
    }

    #[test]
    fn vietnamese_letters_and_tones_are_vietnamese() {
        assert_eq!(guess_language("Xin chào"), Some(Language::vietnamese()));
        assert_eq!(guess_language("Tiếng Việt"), Some(Language::vietnamese()));
        assert_eq!(
            guess_language("Đây là câu trả lời."),
            Some(Language::vietnamese())
        );
        assert_eq!(guess_language("Hà Nội"), Some(Language::vietnamese()));
    }

    #[test]
    fn a_combining_tone_mark_is_vietnamese_evidence() {
        assert_eq!(guess_language("cha\u{0300}o"), Some(Language::vietnamese()));
    }

    #[test]
    fn text_without_a_letter_has_no_guess() {
        assert_eq!(guess_language(""), None);
        assert_eq!(guess_language("42 — 3.14 !?"), None);
        assert_eq!(guess_language("   "), None);
    }

    #[test]
    fn digits_and_punctuation_do_not_hide_the_latin_letters_beside_them() {
        assert_eq!(guess_language("42 apples"), Some(Language::english()));
    }
}
