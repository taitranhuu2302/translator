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
