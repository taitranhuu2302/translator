# 07: Onboarding and notices

**What to build:** The last two surfaces join the system. Onboarding becomes a `Dialog` instead of a backdrop div, and every notice renders through `Alert` with an icon and a colour per cause while `notice.ts` remains the only source of wording.

**Blocked by:** 02, 03

**Status:** resolved

- [x] Onboarding is a `Dialog` with the existing content and the existing one-time behaviour; focus is trapped, Escape closes, and focus returns to the window.
- [x] `NoticeView` renders through `Alert`; each notice kind maps to one icon and one colour (capture, offline, provider, error, ok).
- [x] No component writes failure copy of its own; every sentence still comes from `notice.ts`.
- [x] The single action a notice can carry is still rendered and still performs its IPC call through `noticeActions.ts`.
- [x] Both the popup's failure states and the window's notices use the same rendering.
- [x] The harness renders every notice kind, both resolved and unresolved Accessibility.
- [x] Typecheck, tests and build pass.

## Implementation

- `ui/src/Onboarding.tsx` is a `Dialog`/`DialogContent` with `showCloseButton={false}`.
  Its content and its behaviour are the ones that were there: the four shortcuts
  as `Kbd` + name + action, the Accessibility paragraph, Grant / Open Privacy
  Settings, the grant problem line, and "Got it" writing `onboarding_seen`. A
  save failure still keeps the dialog open and shows the one sentence it already
  owned.
- The semantics changed rather than the words. Focus lands inside the dialog when
  it opens, stays there while it is open, and returns to the window when it
  closes; Escape and the backdrop put it away for this session **without**
  writing the flag, which is what makes "Got it" meaningful. All three were
  checked in the harness (focus on the Grant button, Escape closing, focus back
  on the textarea) rather than assumed from the primitive's documentation.
- `ui/src/NoticeView.tsx`: `Alert` + `AlertTitle` + `AlertDescription`, with one
  icon and one tint per kind — `AlertTriangle` for a capture failure, `WifiOff`
  for offline, `CloudOff` for a provider, `AlertCircle` for an error,
  `CheckCircle2` for the resolved `ok`. The three recoverable kinds share the
  neutral card tint; only `error` carries the destructive border and wash and
  only `ok` the primary one, which is `DESIGN.md`'s status-state table rather
  than five competing colours.
- `NoticeView` takes a `Notice` and renders its title, detail and (when present)
  its one action's label. It has no copy of its own: a grep for the app's failure
  sentences finds them in `ui/src/notice.ts` and in tests, and nowhere else.
- The action is still the model's single action, and pressing it still calls
  `performNoticeAction` from `ui/src/noticeActions.ts` — the popup passes it
  directly, the window passes it through `CaptureNoticeView`.
- `CaptureNoticeView` is the resolving wrapper: when the capture failure is
  about Accessibility and the grant has since been observed, it swaps in
  `accessibilityGrantedNotice(shortcut)` — the model's own resolved words, not a
  sentence written at the call site.
- Both surfaces render the same component. The popup's `failed`,
  `replace_failed` and `nothing_selected` states and the window's capture banner
  are all `NoticeView`; the popup adds no variant.
- The harness's "Notices · all kinds" column draws all five kinds and then the
  Accessibility notice a second time through `CaptureNoticeView` with
  `granted`, so the resolved card is drawn by the resolving path. The old
  hand-written `ok` entry was deleted rather than kept beside it, so the
  harness cannot drift from `notice.ts`.

## Verification

| Command | Result |
| --- | --- |
| `npm --prefix ui run typecheck` | clean |
| `npm --prefix ui run test` | 13 files, 172 tests passed, including `notice.test.ts` (every kind, and the resolved Accessibility wording) and `noticeView.test.ts` |
| `npm --prefix ui run build` | clean |
| `bash scripts/test.sh` | exit 0 |
| Design preview `#notices` | five kinds plus the resolved Accessibility card, each with its own icon and tint |
| Design preview `#onboarding` | dialog centred over the panes; focus inside on open, Escape closes it, focus returns to the textarea |
| `grep` for the app's failure sentences under `ui/src` | only `notice.ts` and tests |

## Comments

- 2026-11-16 — Three notice kinds share the neutral tint on purpose. Five
  distinct colours would have turned a routine missing key into an alarm; the
  icon carries the cause and the tint carries the severity.
- 2026-11-16 — `Dialog`'s generated styling was adjusted once: dialogs now carry
  `shadow-lg` and the cards' radius, like every other floating surface, instead
  of only a 1px ring. Recorded here because the change lives in the generated
  component, not in `Onboarding`.
