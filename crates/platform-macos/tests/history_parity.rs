//! The in-memory history store and the real SQLite one are alternative
//! implementations of one port, so they must answer the same queries the same
//! way. A test that passes against the fake is only evidence about the app if
//! this is true.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use translator_core::testing::InMemoryHistory;
use translator_core::{
    Direction, HistoryEntry, HistoryStore, Language, Mode, ProviderId, Timestamp,
};
use translator_platform_macos::MacHistoryStore;

fn temp_path() -> PathBuf {
    // Unique per call and per process: tests run in parallel and neither the
    // clock nor the temporary directory guarantees an unused path.
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir()
        .join(format!(
            "translator-history-parity-{}-{nanos}-{sequence}",
            std::process::id()
        ))
        .join("history.sqlite3")
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

/// Both stores holding the same rows, recorded oldest first. The rows are
/// chosen so each parity query has something to match and something to reject.
fn stores_with_the_same_rows() -> (InMemoryHistory, MacHistoryStore) {
    let fake = InMemoryHistory::default();
    let real = MacHistoryStore::at(temp_path());
    let rows = [
        entry("100% sure", "chắc chắn", 100),
        entry("a_b", "underscore", 200),
        entry("back\\slash", "escape", 300),
        entry("aXb", "wildcard bait", 400),
        entry("plain", "nothing special", 500),
    ];
    for row in &rows {
        fake.record(row).expect("record into the in-memory store");
        real.record(row).expect("record into the macOS store");
    }
    (fake, real)
}

#[test]
fn both_stores_answer_the_same_queries_the_same_way() {
    let (fake, real) = stores_with_the_same_rows();

    for query in ["", "   ", "100%", "a_b", "\\"] {
        let from_fake = fake.search(query, 50).expect("search the fake");
        let from_real = real.search(query, 50).expect("search the real store");
        assert_eq!(
            from_fake, from_real,
            "the stores disagree on the query {query:?}"
        );
    }
}

#[test]
fn the_parity_queries_have_known_answers() {
    let (fake, _) = stores_with_the_same_rows();

    let sources = |query: &str| -> Vec<String> {
        fake.search(query, 50)
            .expect("search")
            .into_iter()
            .map(|entry| entry.source)
            .collect()
    };
    let newest_first = ["plain", "aXb", "back\\slash", "a_b", "100% sure"];

    // An empty or whitespace-only query is not a filter: it trims to nothing
    // and returns every row.
    assert_eq!(sources(""), newest_first);
    assert_eq!(sources("   "), newest_first);

    // `%`, `_` and `\` are text, not wildcards or an escape: each matches only
    // the row that contains it literally.
    assert_eq!(sources("100%"), ["100% sure"]);
    assert_eq!(sources("a_b"), ["a_b"]);
    assert_eq!(sources("\\"), ["back\\slash"]);
}
