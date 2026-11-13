//! History in SQLite under Application Support, never synced and never sent
//! anywhere. The store has no network dependency at all: its only collaborators
//! are the database file and the rows already in it.
//!
//! The schema is created lazily on the first read or write, so starting the app
//! costs nothing until a translation actually happens. Provider and mode are
//! stored as stable snake_case strings through explicit mapping functions
//! rather than serde, so the on-disk names can never drift with the wire names.

use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use rusqlite::functions::FunctionFlags;
use rusqlite::{params, Connection};
use translator_core::{
    Direction, HistoryEntry, HistoryError, HistoryStore, Language, Mode, ProviderId, Timestamp,
};

const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS history (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        source TEXT NOT NULL,
        result TEXT NOT NULL,
        provider TEXT NOT NULL,
        mode TEXT NOT NULL,
        direction_source TEXT,
        direction_target TEXT,
        corrected TEXT,
        created_at INTEGER NOT NULL
    );
    CREATE INDEX IF NOT EXISTS history_created_at ON history (created_at DESC);
";

const SELECT_COLUMNS: &str = "source, result, provider, mode, direction_source, \
                              direction_target, corrected, created_at";

/// One SQLite database, opened on first use and reused afterwards. A single
/// connection behind a mutex keeps the write path serialized; every operation is
/// a short local statement, never a network call, so the lock is held only for
/// the duration of one query.
pub struct MacHistoryStore {
    path: PathBuf,
    connection: Mutex<Option<Connection>>,
}

impl MacHistoryStore {
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            connection: Mutex::new(None),
        }
    }

    pub fn default_path() -> PathBuf {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default();
        home.join("Library/Application Support/com.taitran.translator/history.sqlite3")
    }

    /// Run one operation against the lazily opened database. A poisoned lock and
    /// an unopenable path are typed failures, never panics.
    fn with_connection<T>(
        &self,
        operation: impl FnOnce(&Connection) -> Result<T, HistoryError>,
    ) -> Result<T, HistoryError> {
        let mut slot = self.lock()?;
        if slot.is_none() {
            *slot = Some(self.open()?);
        }
        let connection = slot.as_ref().ok_or_else(|| HistoryError::Storage {
            message: format!("{}: the database did not open", self.path.display()),
        })?;
        operation(connection)
    }

    fn lock(&self) -> Result<MutexGuard<'_, Option<Connection>>, HistoryError> {
        self.connection
            .lock()
            .map_err(|_| HistoryError::Unavailable {
                reason: "the history database lock was poisoned".to_owned(),
            })
    }

    fn open(&self) -> Result<Connection, HistoryError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| self.storage_error(error))?;
        }
        let connection = Connection::open(&self.path).map_err(|error| self.storage_error(error))?;
        connection
            .execute_batch(SCHEMA)
            .map_err(|error| self.storage_error(error))?;
        register_case_folding(&connection).map_err(|error| self.storage_error(error))?;
        Ok(connection)
    }

    fn storage_error(&self, error: impl std::fmt::Display) -> HistoryError {
        HistoryError::Storage {
            message: format!("{}: {error}", self.path.display()),
        }
    }
}

impl Default for MacHistoryStore {
    fn default() -> Self {
        Self::at(Self::default_path())
    }
}

impl HistoryStore for MacHistoryStore {
    fn record(&self, entry: &HistoryEntry) -> Result<(), HistoryError> {
        self.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO history \
                     (source, result, provider, mode, direction_source, direction_target, corrected, created_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        entry.source,
                        entry.result,
                        provider_name(entry.provider),
                        mode_name(entry.mode),
                        entry
                            .direction
                            .as_ref()
                            .map(|direction| direction.source.code()),
                        entry
                            .direction
                            .as_ref()
                            .map(|direction| direction.target.code()),
                        entry.corrected,
                        entry.created_at.0,
                    ],
                )
                .map_err(|error| self.storage_error(error))?;
            Ok(())
        })
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
        let needle = query.trim().to_lowercase();
        let pattern = like_pattern(&needle);
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);

        self.with_connection(|connection| {
            let sql = format!(
                "SELECT {SELECT_COLUMNS} FROM history \
                 WHERE ?1 = '' \
                    OR utf8_lower(source) LIKE ?2 ESCAPE '\\' \
                    OR utf8_lower(result) LIKE ?2 ESCAPE '\\' \
                 ORDER BY created_at DESC, id DESC LIMIT ?3"
            );
            let mut statement = connection
                .prepare(&sql)
                .map_err(|error| self.storage_error(error))?;
            let mut rows = statement
                .query(params![needle, pattern, limit])
                .map_err(|error| self.storage_error(error))?;

            let mut entries = Vec::new();
            while let Some(row) = rows.next().map_err(|error| self.storage_error(error))? {
                let entry = self.entry_from_row(row)?;
                entries.push(entry);
            }
            Ok(entries)
        })
    }

    fn purge_older_than(&self, cutoff: Timestamp) -> Result<usize, HistoryError> {
        self.with_connection(|connection| {
            connection
                .execute(
                    "DELETE FROM history WHERE created_at < ?1",
                    params![cutoff.0],
                )
                .map_err(|error| self.storage_error(error))
        })
    }
}

impl MacHistoryStore {
    fn entry_from_row(&self, row: &rusqlite::Row<'_>) -> Result<HistoryEntry, HistoryError> {
        let provider: String = row.get(2).map_err(|error| self.storage_error(error))?;
        let mode: String = row.get(3).map_err(|error| self.storage_error(error))?;
        let direction_source: Option<String> =
            row.get(4).map_err(|error| self.storage_error(error))?;
        let direction_target: Option<String> =
            row.get(5).map_err(|error| self.storage_error(error))?;
        let corrected: Option<String> = row.get(6).map_err(|error| self.storage_error(error))?;
        let created_at: i64 = row.get(7).map_err(|error| self.storage_error(error))?;

        Ok(HistoryEntry {
            source: row.get(0).map_err(|error| self.storage_error(error))?,
            result: row.get(1).map_err(|error| self.storage_error(error))?,
            provider: provider_from_name(&provider)?,
            mode: mode_from_name(&mode)?,
            created_at: Timestamp(created_at),
            direction: stored_direction(direction_source, direction_target),
            corrected,
        })
    }
}

/// SQLite's `LIKE` folds ASCII only, so the search folds both sides through the
/// registered `utf8_lower` function: a Vietnamese query in either case finds the
/// row it belongs to.
fn register_case_folding(connection: &Connection) -> rusqlite::Result<()> {
    connection.create_scalar_function(
        "utf8_lower",
        1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        |context| {
            let value: String = context.get(0)?;
            Ok(value.to_lowercase())
        },
    )
}

/// Turn a query into a `LIKE` pattern, escaping `%`, `_` and the escape
/// character so a query containing them is matched literally.
fn like_pattern(needle: &str) -> String {
    let mut pattern = String::with_capacity(needle.len() + 2);
    pattern.push('%');
    for character in needle.chars() {
        match character {
            '\\' => pattern.push_str("\\\\"),
            '%' => pattern.push_str("\\%"),
            '_' => pattern.push_str("\\_"),
            other => pattern.push(other),
        }
    }
    pattern.push('%');
    pattern
}

fn stored_direction(source: Option<String>, target: Option<String>) -> Option<Direction> {
    match (source, target) {
        (Some(source), Some(target)) => Some(Direction {
            source: Language::new(source),
            target: Language::new(target),
        }),
        _ => None,
    }
}

fn provider_name(provider: ProviderId) -> &'static str {
    match provider {
        ProviderId::Azure => "azure",
        ProviderId::MyMemory => "my_memory",
        ProviderId::Groq => "groq",
    }
}

fn provider_from_name(name: &str) -> Result<ProviderId, HistoryError> {
    match name {
        "azure" => Ok(ProviderId::Azure),
        "my_memory" => Ok(ProviderId::MyMemory),
        "groq" => Ok(ProviderId::Groq),
        other => Err(HistoryError::Storage {
            message: format!("history row names an unknown provider: {other}"),
        }),
    }
}

fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Translate => "translate",
        Mode::Improve => "improve",
    }
}

fn mode_from_name(name: &str) -> Result<Mode, HistoryError> {
    match name {
        "translate" => Ok(Mode::Translate),
        "improve" => Ok(Mode::Improve),
        other => Err(HistoryError::Storage {
            message: format!("history row names an unknown mode: {other}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join(format!("translator-history-{nanos}"))
            .join(name)
    }

    fn entry(source: &str, result: &str, created_at: i64) -> HistoryEntry {
        HistoryEntry {
            source: source.to_owned(),
            result: result.to_owned(),
            provider: ProviderId::MyMemory,
            mode: Mode::Translate,
            created_at: Timestamp(created_at),
            direction: Some(Direction {
                source: Language::vietnamese(),
                target: Language::english(),
            }),
            corrected: None,
        }
    }

    #[test]
    fn every_provider_and_mode_round_trips_through_its_stored_name() {
        for provider in [ProviderId::Azure, ProviderId::MyMemory, ProviderId::Groq] {
            assert_eq!(
                provider_from_name(provider_name(provider)).expect("a known provider"),
                provider
            );
        }
        for mode in [Mode::Translate, Mode::Improve] {
            assert_eq!(mode_from_name(mode_name(mode)).expect("a known mode"), mode);
        }
    }

    #[test]
    fn an_unknown_stored_name_is_a_typed_error_instead_of_a_guess() {
        assert!(provider_from_name("nope").is_err());
        assert!(mode_from_name("nope").is_err());
    }

    #[test]
    fn an_empty_database_searches_as_an_empty_list_not_an_error() {
        let store = MacHistoryStore::at(temp_path("history.sqlite3"));

        assert_eq!(
            store.search("", 50).expect("search an empty database"),
            Vec::<HistoryEntry>::new()
        );
    }

    #[test]
    fn record_search_and_purge_round_trip() {
        let store = MacHistoryStore::at(temp_path("history.sqlite3"));
        let mut improved = entry("He go home.", "He's heading home.", 200);
        improved.provider = ProviderId::Groq;
        improved.mode = Mode::Improve;
        improved.direction = None;
        improved.corrected = Some("He is going home.".to_owned());

        store
            .record(&entry("Xin chào", "Hello", 100))
            .expect("record");
        store.record(&improved).expect("record");

        let all = store.search("", 50).expect("search");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].result, "He's heading home.", "newest first");
        assert_eq!(all[0].mode, Mode::Improve);
        assert_eq!(all[0].corrected.as_deref(), Some("He is going home."));
        assert_eq!(all[1].direction, entry("", "", 0).direction);

        let removed = store.purge_older_than(Timestamp(150)).expect("purge");
        assert_eq!(removed, 1);
        assert_eq!(store.search("", 50).expect("search").len(), 1);
    }

    #[test]
    fn a_query_matches_either_side_case_insensitively() {
        let store = MacHistoryStore::at(temp_path("history.sqlite3"));
        store
            .record(&entry("Xin chào", "Hello there", 100))
            .expect("record");

        assert_eq!(store.search("hello", 10).expect("english side").len(), 1);
        assert_eq!(
            store
                .search("XIN CHÀO", 10)
                .expect("upper-case vietnamese")
                .len(),
            1
        );
        assert_eq!(store.search("chào", 10).expect("vietnamese side").len(), 1);
        assert!(store.search("goodbye", 10).expect("no match").is_empty());
    }

    #[test]
    fn wildcard_characters_in_a_query_are_literal() {
        let store = MacHistoryStore::at(temp_path("history.sqlite3"));
        store
            .record(&entry("100% sure", "chắc chắn", 100))
            .expect("record");
        store
            .record(&entry("a_b", "underscore", 101))
            .expect("record");
        store
            .record(&entry("plain", "nothing special", 102))
            .expect("record");

        let percent = store.search("%", 10).expect("percent literal");
        assert_eq!(percent.len(), 1);
        assert_eq!(percent[0].source, "100% sure");

        let underscore = store.search("_", 10).expect("underscore literal");
        assert_eq!(underscore.len(), 1);
        assert_eq!(underscore[0].source, "a_b");

        assert!(store.search("100%", 10).expect("a full literal").len() == 1);
        assert!(store
            .search("\\", 10)
            .expect("the escape character")
            .is_empty());
    }

    #[test]
    fn the_limit_and_newest_first_order_are_respected() {
        let store = MacHistoryStore::at(temp_path("history.sqlite3"));
        for seconds in [1, 2, 3, 4, 5] {
            store
                .record(&entry(&format!("source {seconds}"), "same", seconds))
                .expect("record");
        }

        let newest = store.search("", 3).expect("limited search");
        assert_eq!(newest.len(), 3);
        assert_eq!(newest[0].source, "source 5");
        assert_eq!(newest[1].source, "source 4");
        assert_eq!(newest[2].source, "source 3");
    }

    #[test]
    fn the_schema_is_reused_when_the_database_is_opened_again() {
        let path = temp_path("history.sqlite3");
        {
            let store = MacHistoryStore::at(&path);
            store
                .record(&entry("Xin chào", "Hello", 100))
                .expect("record");
        }

        let reopened = MacHistoryStore::at(&path);
        let entries = reopened.search("", 10).expect("reopen and search");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source, "Xin chào");
    }

    #[test]
    fn a_corrupt_database_is_a_typed_error_not_a_panic() {
        let path = temp_path("history.sqlite3");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"this is not sqlite").unwrap();
        let store = MacHistoryStore::at(&path);

        match store.search("", 10) {
            Err(HistoryError::Storage { message }) => {
                assert!(message.contains("history.sqlite3"), "{message}")
            }
            other => panic!("expected a storage error, got {other:?}"),
        }
    }

    #[test]
    fn an_unwritable_path_is_a_typed_error_not_a_panic() {
        let parent = temp_path("history.sqlite3");
        // A regular file where the directory should be makes `create_dir_all`
        // fail; the store must report that rather than unwrap it.
        fs::create_dir_all(parent.parent().unwrap()).unwrap();
        fs::write(&parent, b"not a directory").unwrap();
        let store = MacHistoryStore::at(parent.join("history.sqlite3"));

        match store.record(&entry("Xin chào", "Hello", 100)) {
            Err(HistoryError::Storage { .. }) => {}
            other => panic!("expected a storage error, got {other:?}"),
        }
    }
}
