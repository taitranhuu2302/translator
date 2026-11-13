# 09: History

**What to build:** Everything the app translates, Replaces or Improves is kept locally and can be found again. Old entries age out on their own, so the list stays useful without anyone maintaining it, and nothing is ever sent anywhere.

**Blocked by:** 07

**Status:** resolved

- [x] Every translate, Replace and Improve is recorded locally with both sides, the provider, the mode and a timestamp
- [x] History is searchable by either side of the Language Pair
- [x] Entries older than 30 days are purged at launch and again daily
- [x] History stays on this Mac; no history content is sent to any provider or sync service
- [x] An entry can be reopened in the Translate Window with its text and direction restored
- [x] With an empty history the surface shows an empty state rather than an error

## Verification

The app was **never launched** while building this ticket. The boxes are proven
by the automated suites below; the parts that need a real window, a real
database file and a network capture are written up in
[`../checks/09-history-manual.md`](../checks/09-history-manual.md).

| Command | Result |
| --- | --- |
| `npm --prefix ui run test` | 61 passed — 14 new: 12 `historyView` tests (either-side matching, case folding, spaces, empty query, no match, order, mode and relative-time labels) and 1 "restored" transition in each result reducer. |
| `cargo test -p translator-core` | 110 library tests passed — 5 new: 3 `HistoryEntry` round trips (direction, both Improve parts, a row written before the new fields) and 2 30-day cutoff tests (exact value, and the boundary where a row exactly 30 days old survives while one second older is purged). |
| `cargo test -p translator-core --test engine_seam` | 61 passed — 13 new or replaced: direction recorded only when known, explicit window direction kept, one row for popup Enter and one for Replace, Improve recorded once through both entry points, a rejected history write never fails a translation or an improvement, reopening both modes without a provider, the saved-target fallback, an old Improve row with no corrected part, and purge through the engine. |
| `cargo test -p translator-platform-macos` | 22 passed — 9 new: name round trips, typed errors for unknown names, empty database, record/search/purge, either-side case-insensitive search including `XIN CHÀO`, literal `%`, `_` and `\`, limit and newest-first order, schema reuse on reopen, corrupt and unwritable paths. |
| `cargo test -p translator-app` | 11 passed — unchanged |
| `cargo clippy --workspace --all-targets` | clean |
| `./scripts/test.sh` | passed (typecheck, vitest, IPC boundary, build, cargo, doctests) |

Proofs for the ticked boxes:

- **Everything is recorded, with both sides, provider, mode and time:** the
  engine writes through the `HistoryStore` port for Quick Translate,
  `Replace`, Translate and both Improve entry points. The platform store keeps
  `source`, `result`, `provider`, `mode`, `direction_source`,
  `direction_target`, `corrected` and `created_at`; its round-trip test reads
  all of them back.
- **Replace records one row, not two:** `replace_with_translation` no longer
  writes history. `InMemoryHistory::records()` counts calls at the seam:
  `a_popup_enter_after_quick_translate_writes_one_history_row_not_two` and
  `replace_selection_records_one_row_through_the_seam`.
- **Searchable by either side:** the store filters
  `utf8_lower(source) LIKE … OR utf8_lower(result) LIKE …`, with the query
  lowered and wildcards escaped. `a_query_matches_either_side_case_insensitively`
  finds a Vietnamese source from `hello` and from `XIN CHÀO`.
- **Thirty-day purge:** `history_cutoff` subtracts exactly
  `HISTORY_RETENTION_SECONDS`; the boundary test proves `created_at == cutoff`
  survives and `cutoff - 1` is deleted. `maintain_history` purges immediately
  and every 24 hours on a background thread, logging failures without blocking
  startup; `purging_history_removes_rows_older_than_thirty_days` proves the
  engine path.
- **Local only:** `crates/platform-macos/src/history.rs` has no HTTP client,
  provider or sync dependency; its collaborators are the SQLite file and the
  rows in it. Search and reopen run entirely in process. The manual checks
  include a `tcpdump` capture.
- **Reopening restores text and direction:** `TranslatorEngine::reopen` shapes
  a `Translation` from the stored row (or an `Improvement` with both parts)
  without reading a provider or recording anything. The two reducers gained a
  `restored` event, and the reducer tests prove the pane fills directly from
  the stored answer without a `translating`/`improving` phase.
- **Empty state:** `HistoryPane` shows exactly "Nothing here yet —
  translations and improvements you make will appear here." whenever the
  filtered list is empty, including a search that matches nothing.

## Decisions

- **`HistoryEntry` grew two optional fields.** `direction: Option<Direction>`
  and `corrected: Option<String>` are both `#[serde(default)]`, so a row
  written before this ticket still loads; the model has a test for that shape.
  All literals in production and the fakes were updated.
- **Direction is recorded only when it is known.** For a window translation the
  explicit `Direction` is kept; otherwise the provider's `detected_source` plus
  the translation target forms it. When the provider detects nothing, the entry
  has `direction: None` rather than a guess. This is what lets reopen fall back
  to the saved pair's target.
- **Improve is recorded exactly once through each entry point.** Both
  `translate_text_in(..., Mode::Improve, ...)` and `improve_streaming` record
  after the provider validates the pair. The old seam test that asserted
  Improve wrote nothing was replaced by the two recording tests and a
  rejected-write test.
- **The duplicate Replace write was removed.** Quick Translate records the
  translation it shows; pressing Enter or Replace afterwards only writes the
  Selection. The `records` counter on the fake port makes the single-write rule
  observable without querying the database.
- **SQLite lives in its own platform module.** `MacHistoryStore` uses
  `rusqlite` with `bundled` (and `functions`) at
  `~/Library/Application Support/com.taitran.translator/history.sqlite3`. The
  schema is created lazily on the first operation. One `Mutex<Option<Connection>>`
  serializes short local statements; a poisoned lock and every open/query
  failure become `HistoryError`, never a panic.
- **Stored names are mapped explicitly.** Provider (`azure`, `my_memory`,
  `groq`) and mode (`translate`, `improve`) have hand-written mapping functions
  with round-trip tests, so the on-disk spelling cannot drift with serde or the
  wire format.
- **Case-insensitive search is Unicode-aware.** SQLite's `LIKE` and `lower()`
  fold only ASCII, so the store registers a `utf8_lower` scalar function and
  applies it to both columns and the lowered query. `%`, `_` and `\` are
  escaped in Rust and matched with `ESCAPE '\'`. Results are newest first and
  respect the limit; an empty query returns the newest entries and an empty
  database returns an empty list.
- **`reopen` returns `TextOutcome`, not a new `Reopened` type.** The UI's
  `translateText` already returns `TextOutcome`, and the two reducers already
  model a translation and an improvement; introducing a third shape would
  duplicate it. A translation recovers `detected_source` from the stored
  direction and the target from that direction, falling back to the saved
  pair's target when no direction was stored. An improvement returns
  `corrected` (empty for a row that never had one) and `result` as `better`.
  The IPC command is `reopen_history_entry`.
- **v1 has one Improve provider.** `record_improvement` stores
  `ProviderId::Groq`, because Groq is the only provider that can Improve and an
  `Improvement` carries no provider. A second Improve provider would have to
  put the answering provider into the result.
- **The History surface filters locally.** It loads at most 500 recent entries
  once (`ipc.searchHistory("", 500)`), then `filterHistory` narrows them as the
  user types — no IPC per keystroke. A click switches to the entry's mode,
  restores its source and a known direction, and fills the pane through the
  `restored` event, so no provider is called and no new row is written.
- **Purge is best-effort and off the startup path.** `maintain_history` runs a
  dedicated thread that purges immediately then sleeps 24 hours; errors are
  logged with `eprintln!`. A slow or unwritable database cannot delay the first
  shortcut.
- **Not verified here:** the rendered History list and search field, restart
  persistence against the real Application Support path, the daily repeat of
  the purge, and the absence of network traffic. Those are the manual checks,
  and the app was not launched.

## Comments

- 2026-11-14 — Implemented in `0f3112d`. A real window, the real SQLite file
  under Application Support, restart persistence and a `tcpdump` privacy
  capture still need a run on the signed bundle:
  [`../checks/09-history-manual.md`](../checks/09-history-manual.md) covers a
  translation and a Replace appearing once, an Improve with both parts, search
  from either side and case-insensitively, literal wildcards, aging a row past
  30 days with SQL, reopening with its direction, reopening without quota or a
  new row, empty and no-match states, restart persistence, and the absence of
  provider traffic while searching or reopening.
