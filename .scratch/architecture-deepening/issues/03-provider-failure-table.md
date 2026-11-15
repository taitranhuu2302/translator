# 03: Provider failure table — one mapping for every rung

**What to build:** One module in the core owns every conversion from a transport or HTTP outcome into a provider error. Each rung supplies its policy as data — which statuses mean authentication, which provider codes map to which authentication cause or to quota, what its server-error family is, and its rate-limit sentence — instead of a fourth copied match. A rung's module keeps only what is genuinely its own: request shape, response decoding, and its code table. User-facing sentences live next to the code table that selects them, not inside the rungs.

Every current per-provider decision is preserved exactly and turned into data: Azure's quota versus resource-not-authorized split on 403 with its fallback, the differing server-error families across rungs, and MyMemory's body-level quota detection. This ticket changes no behaviour; it gives existing behaviour one home.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] Transport failures are mapped in one place for all rungs.
- [x] Rate limits, authentication statuses, server errors, and unrecognised statuses are mapped through one table, with each rung's policy expressed as data.
- [x] The three copied transport mappers are gone, and no rung module formats a user-facing sentence.
- [x] Behaviour is provably unchanged: through the ladder, authentication and quota failures still skip the rung for the session, a rate limit still does not, and every rung's server-error family and message are what they were.
- [x] Tests drive these cases through the ladder rather than through the mappers, so they survive a rewrite of the module's internals.
- [x] No public interface of the core widens: the table is internal to the providers side.

## Implementation

- `crates/core/src/providers/failure.rs` is the one table. `FailurePolicy` is
  the data each rung supplies: auth statuses with their body-code table and
  fallback (`AuthStatus`/`Meaning`), statuses that are temporarily unavailable
  with the provider's sentence, the rate-limit sentence, the server-error range
  and family (`ServerErrors`/`ServerFamily`), the unrecognised-status policy
  (`Unrecognised`/`BodyDetail`) and the body-level quota markers. It owns
  `transport`, `status`, `body_quota` and `unreadable_key`, and
  `impl Display for ProviderError` moved here from `error.rs`, so a provider's
  words sit beside the code table that selects them. `mod failure;` is private
  to `providers`; `lib.rs`'s re-exports are unchanged.
- Each rung declares `const FAILURES: FailurePolicy` and keeps only request
  shape, envelope decoding, and its code table. Azure: 401 → bad key; 403 with
  `403001` → quota, `403000` → resource-not-authorized, anything else → bad key;
  5xx → `Network`. Groq: 401/403 → bad key; 422/498 → its temporary sentence;
  5xx → `Network`. MyMemory: no auth statuses, every status ≥ 500 →
  `Unavailable`, unrecognised statuses fall through to the body, and
  `quotaFinished`/`MYMEMORY WARNING`/`USED ALL AVAILABLE FREE TRANSLATIONS`
  remain quota signals. The rate-limit sentence per rung is unchanged.
- Deleted: the three copied `map_transport` functions; Azure's status `match`,
  `authentication_failure`, `azure_error_code`; Groq's `failure_status`,
  `body_message`, `malformed`; MyMemory's inline 429/5xx/quota branches.
- Decoding diagnostics ("the body was not the expected JSON", "the answer
  carried no translated text") stay in the rungs by design: they describe that
  rung's own envelope, not a provider-selected failure sentence. Every
  provider-selected sentence and the `Display` wording now lives in the table.

## Verification

| Command | Result |
| --- | --- |
| `cargo test -p translator-core --test engine_seam` | 74 passed, 0 failed — including 11 new ladder-driven cases: 401/403 quota/403 resource/403 fallback, 429's sentence and no session skip, each rung's 5xx family and message, Groq 422/498, MyMemory body-level quota, unrecognised statuses, and transport mapping per rung |
| `cargo test -p translator-core` | 194 passed, 0 failed (120 unit + 74 engine seam) |
| `./scripts/test.sh` | exit 0 — typecheck, 9 UI files / 118 tests, production build, IPC boundary, `cargo test --workspace` (30 + 120 + 3 + 74 + 22) |
| `./scripts/test.sh` after merging `spec/architecture-deepening` | exit 0 — 10 UI files / 132 tests; `cargo test --workspace` (34 + 120 + 3 + 74 + 22 + 2); no conflicts |

No manual checks: the ticket changes no wording or runtime behaviour, and the
sentences are asserted through the ladder. The app was not launched.
