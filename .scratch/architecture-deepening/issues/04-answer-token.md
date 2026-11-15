# 04: Answer token — one owner for "the newest answer wins"

**What to build:** One small module owns freshness for every asynchronous answer. It issues a token when an attempt starts and answers whether that token is still current. A token carries both a session identity and a sequence number, so it survives the case a bare counter cannot: the Quick Translate popup is reopened, a new session starts translating, and the previous session's answer arrives afterwards. The popup, the Translate Window's Translate pane, and its Improve pane all store a token instead of inventing their own counter, and the wire's string request id is converted in the IPC module only.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] The token module issues and retires tokens; no other module defines its own request counter.
- [x] The popup reducer drops an answer from a session that is no longer current, including the reopen-while-translating case the current test does not cover.
- [x] The Translate and Improve reducers accept an answer or a chunk only for a current token; a restored history entry stays valid from any phase because it answers no request.
- [x] The IPC module is the only place a token becomes the wire's request id and back.
- [x] The component-level session counter disappears; the reducer's own rule is sufficient on its own.
- [x] Tests cover: a late answer from a previous session, a late chunk from an abandoned attempt, a current answer accepted, and two sequential sessions in the same open popup.

## Implementation

- `ui/src/answerToken.ts` is the one owner. `beginSession()` opens a session
  (allocating the next module-wide session identity) and returns an issuer
  whose `issue()` hands out `{ session, sequence }`; `isCurrent(token, current)`
  compares both fields. A reopened surface calls `beginSession()` again, so a
  previous session's token is never current even when its sequence matches.
- `ui/src/popupState.ts` stores the token in `translating` and drops a
  `translated`/`failed` event whose token is not current. `PopupView.tsx` no
  longer holds a `session` ref: every prefill claims `beginSession().issue()`
  and dispatches that token with the answer it awaits.
- `ui/src/translateWindow.ts` stores the token in `translating`; `started`,
  `translated` and `failed` carry one. `ui/src/improveWindow.ts` stores the
  token in `improving`; `started`, `chunk`, `done` and `failed` carry one.
  `restored` carries no token in either reducer and applies from any phase.
- `ui/src/ipc.ts` owns the conversion: `requestIdFor(token)` writes
  `"<session>:<sequence>"`, `tokenForRequestId` parses it back, and an id this
  build did not mint becomes null (an answer for no attempt). `improveText`
  takes a token; the streaming listeners hand listeners a token-shaped event.
  `App.tsx` holds an `AnswerSession`, not a counter, and compares nothing.
- Wire shapes (`request_id` as a string in the events and the `improve_text`
  argument) are unchanged on both sides.

## Verification

| Command | Result |
| --- | --- |
| `npm --prefix ui run test` | 12 files / 143 tests passed before the merge |
| `./scripts/test.sh` | exit 0 — typecheck, 12 UI files / 143 tests, production build, IPC boundary, `cargo test --workspace` (34 + 120 + 3 + 74 + 24 + 2) |
| `git merge spec/architecture-deepening --no-edit` | clean merge with ticket 02's blank-rule comments and `nothing_to_translate`; no conflicts |
| `./scripts/test.sh` after merging `spec/architecture-deepening` | exit 0 — 12 UI files / 145 tests; `cargo test --workspace` (34 + 121 + 3 + 78 + 24 + 2) |

The tests that pin this down live in `answerToken.test.ts`,
`popupState.test.ts` (late answer from the open replaced mid-translation, the
reopened session's answer accepted while the replaced one is dropped, and a
dismissed popup staying dismissed), `translateWindow.test.ts` (a late answer
from an abandoned attempt), `improveWindow.test.ts` (a late chunk, a late
`done`/`failed`, and an answer after the attempt finished) and `ipc.test.ts`
(round-trip, session distinction, malformed id). No app launch was needed; no
user-facing behaviour or wording changed.
