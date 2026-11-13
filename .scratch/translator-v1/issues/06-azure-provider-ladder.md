# 06: Azure F0 and the provider ladder

**What to build:** The user enters an Azure Translator key once, and from then on translations go through Azure with the keyless provider behind it. When Azure refuses — bad key, exhausted quota — the app still delivers a translation and says which provider answered. This is where the app stops being keyless-only and starts behaving like the provider ladder in the ADR.

**Blocked by:** 04

**Status:** resolved

- [x] A Settings window accepts an Azure key and stores it in the Keychain, never in user defaults or a file in the app bundle
- [x] With a key configured, translation goes through Azure and the result records which provider produced it
- [x] An auth or quota failure at Azure falls back to the keyless provider transparently, and the result still appears
- [x] A provider that failed with an auth or quota error is not retried for the rest of the session
- [x] With no key configured, the app uses the keyless provider without showing an error
- [x] Errors name the provider and say what would fix it

## Verification

The app was **never launched** while building this ticket. The boxes are proven
by the automated suites below; the parts that need a real Keychain, a real Azure
account and the running bundle are written up in
[`../checks/06-azure-manual.md`](../checks/06-azure-manual.md).

| Command | Result |
| --- | --- |
| `npm --prefix ui run test` | 22 passed — provider status wording, region trimming, auth wording for a bad key vs a refused resource |
| `cargo test -p translator-core` | 69 library tests passed — Azure request/response/error mapping, the ladder's session skip |
| `cargo test -p translator-core --test engine_seam` | 42 passed — saving/clearing a key, status with and without a session skip, the key never reaching settings |
| `cargo test -p translator-platform-macos` | 12 passed — Keychain account mapping |
| `cargo test -p translator-app` | 2 passed — shortcut routing |
| `cargo clippy --workspace --all-targets` | clean |
| `./scripts/test.sh` | passed (typecheck, vitest, IPC boundary, build, cargo, doctests) |

Proofs for the ticked boxes:

- **The key is stored outside the settings file:** `saving_a_key_stores_it_trimmed_and_forgets_the_session_skip`
  proves the engine writes only through `CredentialStore`; `saving_a_blank_key_clears_the_stored_key_without_touching_the_settings_file`
  proves the settings document is byte-for-byte unchanged; `each_provider_has_its_own_account_under_the_fixed_service`
  pins the Keychain service and accounts. That macOS really persists it and the
  app reads it back is manual cases 1 and 7.
- **Azure is used and the result records it:** `a_successful_answer_is_the_translated_text_with_the_provider_and_detection`
  (URL, key header, body, `provider: Azure`, detected language) and
  `the_first_provider_that_answers_wins_and_the_rest_are_not_asked` (MyMemory is
  never asked). The popup already draws `providerLabel(answer.provider)`. The
  real Azure answer is manual case 3.
- **Auth/quota failures fall through transparently:**
  `an_authentication_failure_is_never_retried_that_session` and
  `a_quota_failure_is_never_retried_that_session` show the second rung answering
  while the user sees a translation, not an error. Manual cases 4–5 confirm the
  real dialogs stay quiet on the fallback path.
- **A failed provider is not retried that session:**
  `an_authentication_failure_is_never_retried_that_session` (Azure asked once,
  MyMemory twice), `a_quota_failure_is_never_retried_that_session`,
  `provider_status_reports_which_providers_the_session_has_skipped`, and the
  negative tests `an_offline_failure_leaves_the_provider_in_the_ladder` /
  `a_rate_limit_leaves_the_provider_in_the_ladder` /
  `a_malformed_response_leaves_the_provider_in_the_ladder`. Manual case 4.
- **No key means no error:** `no_key_is_not_configured_without_touching_the_network`
  (no HTTP request at all) and `a_missing_key_skips_nothing_and_falls_through_without_an_error`.
  Manual case 1.
- **Errors name the provider and the fix:**
  `a_bad_key_names_the_provider_and_the_fix`,
  `a_disabled_resource_points_at_the_resource_not_the_quota`, and the vitest
  `a bad key names the provider and points at Settings` /
  `a refused Azure resource points at the resource, never at billing quota`.
  The Replace dialog shows the same `Display` string through
  `show_replace_failure`. Manual cases 4–6.

## Decisions

- **`or_any_status` is the real client, not the sketch.** The Azure notes' Rust
  sketch expects `Err` carrying the status, but `crates/core/src/http.rs` uses
  ureq's `.or_any_status()`, so every status arrives as `Ok(HttpResponse)`. The
  provider therefore reads `status` and `body` itself and maps them; no HTTP
  change was needed. The error table still lands exactly as specified.
- **The cause is typed, not parsed from prose.** `ProviderError::AuthenticationFailed`
  gained `cause: AuthFailure` (`invalid_key` / `resource_not_authorized`), with
  `#[serde(default)]` so older payloads read as a bad key. A 401 is a bad key; a
  403/403000 is a resource refusal (disabled, suspended, wrong region); a 403
  with no recognised code stays a bad key, never a quota problem. The two causes
  have different fixes, so they cannot share one message.
- **429 is a retry, not a ban.** `QuotaExceeded` and `AuthenticationFailed` end a
  provider's session; `Offline`, `Network`, `MalformedResponse` and `Unavailable`
  do not. `Unavailable` carries 429 ("try again shortly") and must leave the rung
  in the ladder, which the rate-limit test pins.
- **The ladder owns the skip, keyed by explicit rung id.** `ProviderLadder::new`
  takes `(ProviderId, Arc<dyn TranslationProvider>)`; a `Mutex<Vec<ProviderId>>`
  holds the session's skips. `skipped()` and `clear_skip` were added to the
  provider port with empty defaults and forwarded through `CachedProvider`, so
  `provider_status` and the Settings surface can read them without making every
  provider know its own identity.
- **Saving or clearing a key forgets the skip.** Otherwise fixing a rejected key
  in Settings would do nothing until the app restarted. An empty or whitespace-only
  key means "clear", so Settings needs one field and one rule.
- **No chunking.** Azure accepts 50,000 characters per request and the app never
  sends a Selection near that; MyMemory keeps its 500-byte chunking.
- **Region is read per request.** The provider takes an `Arc<dyn Fn() -> String>`
  that loads settings, so editing the region takes effect without a restart, and
  `AzureProvider` stays independent of `SettingsStore`. An empty region sends no
  `Ocp-Apim-Subscription-Region` header (a global resource rejects one).
- **The key never leaves the Keychain.** `MacCredentialStore` uses
  `security-framework` 3.7 (platform-macos only): service
  `com.taitran.translator`, accounts `azure`/`groq`/`mymemory`. A missing item is
  `Ok(None)`; `delete` is idempotent; reads happen lazily at request time so a
  saved key applies to the next shortcut. Nothing logs, serializes or echoes the
  secret; the key field in the UI is a write-only draft.
- **Settings surface is deliberately small, and shaped for ticket 10.** A
  collapsible "Provider keys" section in the Translate Window shows configured /
  no key / skipped this session, takes and clears an Azure key, and persists the
  optional region. The rules live in `ui/src/providerStatus.ts` so ticket 10 can
  move the section into the full Settings view without changing behaviour.
- **Attribution in the Translate Window waits for ticket 07.** The popup names
  the answering provider today. The Translate Window's translation pane is still
  the ticket 07 placeholder, so there is no per-answer provider to show there
  yet; the provider section shows which provider *will* answer. This is the one
  piece of the "attribution reaches the UI" requirement that is not visible in
  both surfaces yet.
- **Unverified, and left to the manual run.** That the Keychain item survives a
  rebuild and re-sign, that the login Keychain prompt/access behaves on a second
  machine, the exact Azure error bodies for a disabled resource, and whether the
  user's resource is regional or global (which decides whether the region header
  helps or hurts).

## Comments

- 2026-11-14 — Implemented in `eb35b48`. The Keychain persistence, real Azure
  answers, bad-key/quota fallbacks and error wording still need a run on the
  signed bundle: [`../checks/06-azure-manual.md`](../checks/06-azure-manual.md)
  covers keyless, valid key, bad key, exhausted quota, disabled resource, region,
  restart persistence, key absence from disk and logs, clearing the key, and
  offline.
