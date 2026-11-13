use serde::{Deserialize, Serialize};

use crate::language::{Language, LanguagePair};

/// The text the user highlighted in another app (or in our own window) when a
/// shortcut fired.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    pub text: String,
}

impl Selection {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

/// Which engine operation produced a result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Translate,
    Improve,
}

/// Which upstream answered a translation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderId {
    Azure,
    MyMemory,
    Groq,
}

/// What the provider port hands back: the translation plus who answered it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderTranslation {
    pub text: String,
    pub provider: ProviderId,
    pub detected_source: Option<Language>,
}

/// A finished translation together with the direction it was resolved to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Translation {
    pub text: String,
    pub provider: ProviderId,
    pub detected_source: Option<Language>,
    pub target: Language,
}

/// The two labelled parts Improve returns. They are separate fields, never prose
/// the view has to parse.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Improvement {
    pub corrected: String,
    pub better: String,
}

/// One row in local history: both sides of a translation, the provider, the mode
/// and when it happened.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub source: String,
    pub result: String,
    pub provider: ProviderId,
    pub mode: Mode,
    pub created_at: Timestamp,
}

/// Seconds since the Unix epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(pub i64);

/// The pair, the shortcut bindings, the start-at-login preference and whether the
/// first-launch onboarding has been dismissed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub language_pair: LanguagePair,
    pub shortcuts: ShortcutBindings,
    pub start_at_login: bool,
    /// Written once the user dismisses the onboarding. Settings files written
    /// before onboarding existed load as `false`, which is what first launch means.
    #[serde(default)]
    pub onboarding_seen: bool,
}

/// User-editable key combinations, stored as their display strings so a later
/// ticket can parse them into Carbon key codes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShortcutBindings {
    pub quick_translate: String,
    pub replace: String,
    pub read_aloud: String,
    pub translate_window: String,
}

impl Default for ShortcutBindings {
    fn default() -> Self {
        Self {
            quick_translate: "Ctrl+Alt+D".into(),
            replace: "Ctrl+Alt+R".into(),
            read_aloud: "Ctrl+Alt+S".into(),
            translate_window: "Ctrl+Alt+E".into(),
        }
    }
}

/// Which providers have an API key in the credential store.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub azure_configured: bool,
    pub groq_configured: bool,
}

/// The outcome of Quick Translate: what was selected and what it became.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuickTranslation {
    pub source_text: String,
    pub translation: Translation,
}

/// The outcome of Replace: the Selection has been overwritten.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replacement {
    pub source_text: String,
    pub translation: Translation,
}

/// The outcome of Read Aloud, which speaks the Selection and never a translation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadAloud {
    pub spoken_text: String,
}

/// The result of a Translate Window request. Streaming arrives in a later ticket;
/// the two modes already share one shape so the views have one code path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextOutcome {
    Translation(Translation),
    Improvement(Improvement),
}
