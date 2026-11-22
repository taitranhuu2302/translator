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

/// A finished translation together with the direction it ran in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Translation {
    pub text: String,
    pub provider: ProviderId,
    pub detected_source: Option<Language>,
    /// The language the text was translated from, as the engine resolved it
    /// before the request. Unlike `detected_source`, this is never a provider's
    /// opinion; it is what the Direction actually was.
    pub source: Language,
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

/// The pair, the shortcut bindings, the start-at-login preference, whether the
/// first-launch onboarding has been dismissed, and whether a Release build looks
/// for a newer one by itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Whether a Release build looks for a newer Release at launch and every six
    /// hours (docs/adr/0009-self-signed-distribution-and-in-app-updates.md). A
    /// local build has no updater, so this changes nothing for it. Files written
    /// before the updater existed load as `true`, the shipped value.
    #[serde(default = "check_updates_by_default")]
    pub auto_check_updates: bool,
    /// Whether finished translations and improvements are written to History.
    /// Off by default, so nothing is kept unless the user asks for it. It only
    /// stops new rows: entries already written stay readable and searchable, and
    /// turning this off never deletes any of them.
    #[serde(default = "save_history_by_default")]
    pub save_history: bool,
    /// The version of the shipped shortcut defaults this file has been upgraded
    /// to. A file written before the defaults changed has no version and reads as
    /// 0, which is what asks for the one-time upgrade; `Default` starts at the
    /// current version, so a fresh install never runs it.
    #[serde(default)]
    pub shortcuts_version: u8,
}

/// The shipped value: on. Both [`Settings::default`] and the serde default read
/// it from here, so a fresh install and an old settings file agree.
fn check_updates_by_default() -> bool {
    true
}

/// The shipped value: off. Both [`Settings::default`] and the serde default read
/// it from here, so a fresh install and an old settings file agree.
fn save_history_by_default() -> bool {
    false
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language_pair: LanguagePair::default(),
            shortcuts: ShortcutBindings::default(),
            start_at_login: false,
            azure_region: String::new(),
            onboarding_seen: false,
            auto_check_updates: check_updates_by_default(),
            save_history: save_history_by_default(),
            shortcuts_version: Self::SHORTCUT_DEFAULTS_VERSION,
        }
    }
}

impl Settings {
    /// The shortcut-default version this build writes.
    pub const SHORTCUT_DEFAULTS_VERSION: u8 = 1;

    /// Move bindings that still hold the previous defaults to the current ones.
    ///
    /// The defaults moved from `⌃⌥` to `⌥⇧`, which one hand can press without
    /// reaching into the keyboard's corner. A file that still holds the old
    /// combinations was never customized, so it follows; a combination the user
    /// chose is left exactly as it is. Runs once per file: the version is written
    /// back, so setting a shortcut to an old default later is never undone by the
    /// next launch.
    ///
    /// Returns whether the settings changed and should be written back.
    pub fn upgrade_shortcut_defaults(&mut self) -> bool {
        if self.shortcuts_version >= Self::SHORTCUT_DEFAULTS_VERSION {
            return false;
        }

        let current = ShortcutBindings::default();
        move_untouched(
            &mut self.shortcuts.quick_translate,
            "Ctrl+Alt+D",
            &current.quick_translate,
        );
        move_untouched(&mut self.shortcuts.replace, "Ctrl+Alt+R", &current.replace);
        move_untouched(
            &mut self.shortcuts.read_aloud,
            "Ctrl+Alt+S",
            &current.read_aloud,
        );
        move_untouched(
            &mut self.shortcuts.translate_window,
            "Ctrl+Alt+E",
            &current.translate_window,
        );
        self.shortcuts_version = Self::SHORTCUT_DEFAULTS_VERSION;
        true
    }
}

/// One binding of the previous defaults, moved to the current default unless the
/// user chose something else.
fn move_untouched(binding: &mut String, previous: &str, current: &str) {
    if binding == previous {
        *binding = current.to_owned();
    }
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
    /// `⌥⇧`: Option and Shift sit together under one hand, where `⌃⌥` put
    /// Control in the keyboard's far corner. A settings file written with the
    /// old defaults is migrated by [`Settings::upgrade_shortcut_defaults`].
    fn default() -> Self {
        Self {
            quick_translate: "Alt+Shift+D".into(),
            replace: "Alt+Shift+R".into(),
            read_aloud: "Alt+Shift+S".into(),
            translate_window: "Alt+Shift+E".into(),
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
    use super::{
        utf16_length, HistoryEntry, Mode, ProviderId, Settings, ShortcutBindings, Timestamp,
    };
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

    /// A file written with the previous defaults and no version field.
    fn settings_written_with_the_old_defaults() -> Settings {
        Settings {
            shortcuts: ShortcutBindings {
                quick_translate: "Ctrl+Alt+D".to_owned(),
                replace: "Ctrl+Alt+R".to_owned(),
                read_aloud: "Ctrl+Alt+S".to_owned(),
                translate_window: "Ctrl+Alt+E".to_owned(),
            },
            shortcuts_version: 0,
            ..Settings::default()
        }
    }

    #[test]
    fn untouched_old_defaults_follow_the_new_ones_once() {
        let mut settings = settings_written_with_the_old_defaults();

        assert!(settings.upgrade_shortcut_defaults());

        assert_eq!(settings.shortcuts, ShortcutBindings::default());
        assert_eq!(
            settings.shortcuts_version,
            Settings::SHORTCUT_DEFAULTS_VERSION
        );
    }

    #[test]
    fn a_combination_the_user_chose_survives_the_upgrade() {
        let mut settings = settings_written_with_the_old_defaults();
        settings.shortcuts.replace = "Cmd+Shift+K".to_owned();

        settings.upgrade_shortcut_defaults();

        assert_eq!(settings.shortcuts.replace, "Cmd+Shift+K");
        assert_eq!(settings.shortcuts.quick_translate, "Alt+Shift+D");
    }

    #[test]
    fn the_upgrade_never_runs_twice_so_an_old_default_can_be_chosen_again() {
        let mut settings = Settings::default();
        settings.shortcuts.quick_translate = "Ctrl+Alt+D".to_owned();

        assert!(!settings.upgrade_shortcut_defaults());
        assert_eq!(settings.shortcuts.quick_translate, "Ctrl+Alt+D");
    }

    #[test]
    fn a_settings_file_written_before_the_version_field_asks_for_the_upgrade() {
        let loaded: Settings = serde_json::from_str(
            r#"{"language_pair":{"target":"en","partner":"vi"},"shortcuts":{"quick_translate":"Cmd+Shift+K","replace":"Cmd+Shift+R","read_aloud":"Cmd+Shift+S","translate_window":"Cmd+Shift+E"},"start_at_login":false}"#,
        )
        .expect("the older settings shape still deserializes");

        assert_eq!(loaded.shortcuts_version, 0);
    }

    #[test]
    fn a_settings_file_written_before_the_history_preference_does_not_record() {
        let loaded: Settings = serde_json::from_str(
            r#"{"language_pair":{"target":"en","partner":"vi"},"shortcuts":{"quick_translate":"Cmd+Shift+K","replace":"Cmd+Shift+R","read_aloud":"Cmd+Shift+S","translate_window":"Cmd+Shift+E"},"start_at_login":false}"#,
        )
        .expect("the older settings shape still deserializes");

        assert!(!loaded.save_history);
        assert!(!Settings::default().save_history);
    }
}
