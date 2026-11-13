# 08: Improve

**What to build:** The Improve tab takes an English sentence the user wrote and returns two things: the same sentence corrected, and a better version of it. The text streams in as the model produces it, and the two parts are separated so either can be copied on its own.

**Blocked by:** 06, 07

**Status:** resolved

- [x] Improve returns the corrected sentence and the better sentence as two clearly separated parts, never as unflagged prose
- [x] The corrected part preserves the author's meaning and register; the better part is more natural or more concise
- [x] Text streams into the pane while the model is answering
- [x] Malformed model output produces an error instead of mixing the two parts together
- [x] Without the key Improve needs, the app says which key is missing instead of failing silently
- [x] A failed Improve leaves the user's input intact

## Verification

The app was **never launched** while building this ticket. The boxes are proven
by the automated suites below; the parts that need a real model answer, a
throttled network and the signed bundle are written up in
[`../checks/08-improve-manual.md`](../checks/08-improve-manual.md).

| Command | Result |
| --- | --- |
| `npm --prefix ui run test` | 47 passed — 13 new for this ticket: 10 Improve-pane transitions and 3 Groq status-line rules. |
| `cargo test -p translator-core` | 105 library tests passed — 24 new Groq provider tests, 9 incremental-extractor tests, 2 ladder tests (fall-through and never a session skip on a missing key), 1 missing-key display test. |
| `cargo test -p translator-core --test engine_seam` | 49 passed — 5 new: streamed deltas reconstruct both parts in order, malformed output is typed, a missing Groq key names Groq, blank input never calls a provider, and Improve writes no history. |
| `cargo test -p translator-platform-macos` | 12 passed — unchanged |
| `cargo test -p translator-app` | 11 passed — unchanged |
| `cargo clippy --workspace --all-targets` | clean — `UreqHttpClient::send` maps ureq's transport error itself, so `result_large_err` does not fire |
| `./scripts/test.sh` | passed (typecheck, vitest, IPC boundary, build, cargo, doctests) |

Proofs for the ticked boxes:

- **Two separated parts, never prose:** `Improvement` keeps `corrected` and
  `better` as separate fields end to end. The provider's strict
  `serde_json::from_str::<Improvement>` on the accumulated document is the only
  authority; the streamed text is progress and `done` replaces both fields in
  the pane. `improve_text_returns_the_two_labelled_parts`,
  `one_feed_reports_the_whole_document` and the 10 `improveWindow` tests pin it.
- **Meaning/register and naturalness:** the prompt asks for exactly that, and
  the strict parse rejects anything that is not the two-field shape rather than
  guessing. The judgement itself is a model property; manual cases 5–7 are the
  human check.
- **Streaming:** `UreqHttpClient::stream` reads the SSE body in 4 KB chunks; the
  provider tests feed the same document split at every byte boundary, one byte
  at a time, with `\r\n`, comments, `[DONE]` with and without a trailing
  newline, and a usage-only final chunk. `every_byte_boundary_reconstructs_both_fields`
  and `one_byte_at_a_time_never_emits_a_split_character` prove no character or
  escape is ever decoded half-way. A `FakeHttpClient` scripts byte chunks, so no
  test opens a socket.
- **Malformed output is typed:** a failed final parse, a missing field, a blank
  field, `finish_reason: "length"`, an abrupt end without `finish_reason`, a
  non-JSON event and an unreachable stream all become `ProviderError` values;
  `malformed_model_output_is_a_typed_provider_error_not_a_guessed_answer`
  proves the engine path.
- **The missing key names its provider:** `ProviderError::MissingKey { provider }`
  displays "Groq needs an API key. Add one in Settings."; the ladder falls
  through it and never marks the session skipped.
  `a_missing_groq_key_names_groq_in_the_engine_error` asserts the exact message,
  and the provider's keyless tests assert the HTTP fake saw no request.
- **A failure keeps the input:** `ImproveResult` never holds the source text;
  chunk, done and failure events only touch the pane. The component's textarea
  is unchanged on every path, and manual case 18 re-checks it against a real
  network failure.

## Decisions

- **Streaming is an optional method on the HTTP port.** `HttpClient` gained
  `stream(request, on_chunk)`; its default calls `execute` and hands the whole
  body once, and `UreqHttpClient` overrides it with `into_reader()` and 4 KB
  reads behind `.or_any_status()` so a non-2xx body is still streamed for the
  provider to map. `FakeHttpClient` scripts byte chunks and failure modes.
- **The Groq key is read per request, never at construction.** `GroqProvider`
  holds only the HTTP client and the `CredentialStore`; `key()` runs when a
  request is about to be made. An absent or blank key returns
  `MissingKey { provider: Groq }` before any request is built, so a keyless
  Improve cannot touch the network. The key is only placed in the
  `Authorization` header and is never logged or echoed.
- **`json_object`, not `json_schema`, and the final parse is the authority.**
  Groq cannot stream a strict `json_schema`, so the request asks for
  `{"type":"json_object"}` with `reasoning_format: "parsed"` and the reply is
  accumulated. `improve_stream.rs` scans that document incrementally — keys in
  either order, escapes and surrogate pairs split across chunks, a key named
  inside another field ignored — and emits only decoded field text. The strict
  parse at the end still decides whether the answer is accepted.
- **SSE parsing buffers bytes, not characters.** Lines are split on `\n` before
  UTF-8 decoding, so a multi-byte character split across reads is never decoded
  half-way. Blank lines and `:` comments are ignored; `[DONE]` stops the stream
  cleanly; a trailing line without a newline is processed after the reader ends.
- **A truncated answer is an error, not a partial pair.** `finish_reason` must
  be `stop`; `length`, a missing reason, or an abruptly ended stream becomes a
  typed failure. The extractor could have shown a partial document, but the
  ticket says malformed output must not be presented as an answer.
- **Improve is deliberately not written to history.** Ticket 09 owns all history
  recording, including the mode column. `engine_seam.rs` has
  `improve_is_not_recorded_in_history_until_the_history_ticket_owns_it`, which
  should be replaced when 09 lands.
- **Whitespace-only input answers two empty parts and the pane clears.** The
  engine returns `Improvement { corrected: "", better: "" }` without calling a
  provider or the Keychain, and the UI clears to its empty prompt rather than
  showing two blank labelled sections. Both are tested.
- **The ladder's default streaming method falls back to `improve`.** A provider
  whose `improve` is `NotConfigured` therefore still falls through correctly;
  `ProviderLadder` overrides `improve_streaming` so the rung that answers gets
  the real sink, and `CachedProvider` forwards it so deltas are not lost behind
  the cache. Groq implements `improve()` as the same streamed path with a no-op
  sink, so the two entry points cannot disagree.
- **The mode switch keeps each pane's state and each stream alive.** Switching
  to Translate does not bump the request id, so an Improve already running keeps
  filling its own sections; only a new request (or emptying the input) abandons
  it. `done` replaces both fields; a failure replaces the pane with words and
  never clears the input.
- **A minimal Groq key field was added to Provider keys.** Without it the feature
  could not be tried at all on a fresh install. Ticket 10 still owns the full
  Settings view; this is the same save/clear surface Azure already had, with the
  status wording kept pure in `providerStatus.ts`.
- **Not verified here:** a real Groq answer, ureq's streaming timing on a live
  network, the model's actual prose, and every visual property of the two
  sections. Those are the manual checks, and the app was not launched.

## Comments

- 2026-11-14 — Implemented in `101b9fc`. The real model, throttled streaming and
  clipboard behaviour still need a run on the signed bundle:
  [`../checks/08-improve-manual.md`](../checks/08-improve-manual.md) covers the
  mode switch, visible streaming, register and naturalness, the two parts and
  their independent Copy buttons, empty and whitespace input, a missing Groq
  key (and the absence of a network request), malformed model output through a
  local SSE stub, a truncated answer, a network failure keeping the input,
  switching modes mid-stream, and a fresh opening starting in Translate.
