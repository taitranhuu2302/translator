use serde::{Deserialize, Serialize};

use crate::error::CaptureError;
use crate::language::{Direction, Language, LanguagePair};

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

impl std::fmt::Display for ProviderId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            ProviderId::Azure => "Azure",
            ProviderId::MyMemory => "MyMemory",
            ProviderId::Groq => "Groq",
        })
    }
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

/// Which half of an `Improvement` a streamed chunk belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImprovePart {
    Corrected,
    Better,
}

/// A piece of one improved field, decoded from the model's JSON as it arrived.
/// The view appends it to the labelled section; it never receives prose it has
/// to split itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImproveDelta {
    pub part: ImprovePart,
    pub text: String,
}

/// One row in local history: both sides of a translation, the provider, the mode
/// and when it happened. `direction` is known when the provider detected the
/// source language or the window translated in an explicit direction, and
/// `corrected` carries Improve's corrected part (`result` holds the better one).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub source: String,
    pub result: String,
    pub provider: ProviderId,
    pub mode: Mode,
    pub created_at: Timestamp,
    /// The direction this result was produced in, when it was known. An entry
    /// written before this field existed loads with no direction.
    #[serde(default)]
    pub direction: Option<Direction>,
    /// Improve's corrected sentence. `None` for a translation, and for rows
    /// written before Improve recorded both parts.
    #[serde(default)]
    pub corrected: Option<String>,
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
    /// The Azure resource's region, e.g. `westeurope`. Empty means a global
    /// resource, in which case no `Ocp-Apim-Subscription-Region` header is sent.
    #[serde(default)]
    pub azure_region: String,
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

/// Which providers have an API key in the credential store, and which the
/// session has skipped after an auth or quota failure. A skipped provider is not
/// tried again until the app restarts or its key is saved again.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub azure_configured: bool,
    pub groq_configured: bool,
    #[serde(default)]
    pub azure_skipped: bool,
    #[serde(default)]
    pub groq_skipped: bool,
}

/// What the Quick Translate popup starts with, captured before the panel is
/// shown while the source app still has focus. "Nothing selected" is its own
/// case so the popup can say "select something first" instead of showing an
/// empty translating state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PopupPrefill {
    Selection(Selection),
    NothingSelected,
    Failure(CaptureError),
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

/// The length of `text` in UTF-16 code units: the unit `AXSelectedTextRange`
/// and `NSString` count in. It is the right length for a text range after
/// Replace — the UTF-8 byte count overshoots Vietnamese, and the scalar count
/// undershoots anything outside the BMP.
pub fn utf16_length(text: &str) -> usize {
    text.encode_utf16().count()
}

#[cfg(test)]
mod tests {
    use super::{utf16_length, HistoryEntry, Mode, ProviderId, Timestamp};
    use crate::language::{Direction, Language};

    #[test]
    fn utf16_length_counts_code_units_not_bytes_or_scalars() {
        assert_eq!(utf16_length(""), 0);
        assert_eq!(utf16_length("Hello"), 5);

        // Vietnamese diacritics are single UTF-16 code units but two or three
        // UTF-8 bytes, so the byte count is the wrong length for a text range.
        let vietnamese = "Xin chào thế giới";
        assert_eq!(utf16_length(vietnamese), vietnamese.chars().count());
        assert!(utf16_length(vietnamese) < vietnamese.len());

        // An emoji outside the BMP is one scalar but two UTF-16 code units, so
        // the scalar count is wrong too. "👋" + " " + "Hello" = 2 + 1 + 5.
        let emoji = "👋 Hello";
        assert_eq!(utf16_length(emoji), 8);
        assert_ne!(utf16_length(emoji), emoji.chars().count());
        assert_ne!(utf16_length(emoji), emoji.len());
    }

    fn improved_entry() -> HistoryEntry {
        HistoryEntry {
            source: "He go home.".to_owned(),
            result: "He's heading home.".to_owned(),
            provider: ProviderId::Groq,
            mode: Mode::Improve,
            created_at: Timestamp(1_700_000_000),
            direction: None,
            corrected: Some("He is going home.".to_owned()),
        }
    }

    #[test]
    fn a_history_entry_round_trips_with_its_direction_and_corrected_part() {
        let entry = HistoryEntry {
            source: "Xin chào".to_owned(),
            result: "Hello".to_owned(),
            provider: ProviderId::MyMemory,
            mode: Mode::Translate,
            created_at: Timestamp(1_700_000_000),
            direction: Some(Direction {
                source: Language::vietnamese(),
                target: Language::english(),
            }),
            corrected: None,
        };

        let json = serde_json::to_string(&entry).expect("a history entry serializes");
        let loaded: HistoryEntry =
            serde_json::from_str(&json).expect("a history entry deserializes");

        assert_eq!(loaded, entry);
    }

    #[test]
    fn an_improvement_keeps_both_parts_after_a_round_trip() {
        let entry = improved_entry();

        let json = serde_json::to_string(&entry).expect("an improvement serializes");
        let loaded: HistoryEntry =
            serde_json::from_str(&json).expect("an improvement deserializes");

        assert_eq!(loaded.result, "He's heading home.");
        assert_eq!(loaded.corrected.as_deref(), Some("He is going home."));
    }

    #[test]
    fn a_history_entry_written_before_the_new_fields_still_loads() {
        let loaded: HistoryEntry = serde_json::from_str(
            r#"{"source":"Xin chào","result":"Hello","provider":"my_memory","mode":"translate","created_at":1700000000}"#,
        )
        .expect("the older shape still deserializes");

        assert_eq!(loaded.direction, None);
        assert_eq!(loaded.corrected, None);
        assert_eq!(loaded.mode, Mode::Translate);
    }
}
