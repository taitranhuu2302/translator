# 03: Read Aloud

**What to build:** Selecting English text in any app and pressing the Read Aloud shortcut speaks it in the system voice. The Slice is complete on its own: it needs no translation provider, no key, and no network, which is why it comes before anything that talks to a server.

**Blocked by:** 02

**Status:** resolved

- [ ] Selecting English text and pressing `⌃⌥S` speaks the Selection with the system speech voice — **manual**: the engine path and the shortcut routing are automated, but only a human can hear the voice (manual case 1)
- [x] Read Aloud speaks only the Selection and never a translation
- [x] With nothing selected, the shortcut produces no speech and no error dialog — the engine answers success with empty spoken text (automated); the absence of a dialog is checked by hand in manual case 3
- [ ] Speech does not block the app: another shortcut still works while it is speaking, and the shortcut works again after it ends — **manual**: the bridge returns as soon as the utterance is queued, but that is a runtime observation (manual cases 4–6)
- [ ] With a Vietnamese Selection, it speaks with a Vietnamese voice when one is available and fails visibly when none is — partly automated: the heuristic, the language the port is asked for, and the typed `NoVoice` failure are tested; hearing the voice and seeing the dialog are manual (manual cases 7–8)

## Implementation

- `SpeechOutput::speak` now takes the resolved `Language` beside the text, because
  the voice depends on it, and `SpeechError::NoVoice { language }` is the typed
  failure for a language the system has no voice installed for. It serialises as
  `{"no_voice": {"language": "vi"}}` and the IPC types mirror it.
- The language comes from `language::guess_language`, a small local heuristic:
  Vietnamese when the text carries Vietnamese letters or tone marks
  (`ă â đ ê ô ơ ư`, the five tones, precomposed or combining), English for other
  Latin text, `None` when there is no letter it can judge. It is not the provider
  ladder's detection (ticket 06) and does not touch `resolve_translation`. Where
  it returns `None` (digits, punctuation, an unknown script), Read Aloud speaks in
  English — the system voice that is always present — rather than failing.
- `TranslatorEngine::read_aloud` maps `NothingSelected` to success with empty
  spoken text and speaks nothing, guards blank text the same way, and leaves a
  missing Accessibility grant, secure input and an unavailable capture as typed
  errors. It never calls the provider port.
- `MacSpeechOutput` is real now: `src/macos/speech.m` drives
  `AVSpeechSynthesizer` behind the existing Objective-C bridge pattern (compiled by
  `build.rs`, wrapped by `src/speech.rs`), with AVFoundation linked. The bridge
  resolves a voice by exact tag, then by tag prefix (`vi` → `vi-VN`), then by base
  language, or answers `TL_SPEECH_NO_VOICE`. Speaking is handed to the main queue
  and the call returns at once; a new utterance stops the previous one, and the
  newest utterance is spoken when the cancellation completes, which is what makes
  a second press while it is still talking work instead of being dropped.
- `⌃⌥S` is registered from `settings.shortcuts.read_aloud` in the shell. The single
  global-shortcut handler now routes by the key combination of the shortcut it was
  given, so the Translate Window binding and the Read Aloud binding do not collide;
  a binding that failed to register routes nowhere. Read Aloud runs on a worker
  thread so capture and speech never block the next shortcut.
- A typed Read Aloud failure is shown with a native message dialog
  (`tauri-plugin-dialog`). The dialog is the deviation from ticket 02's shell: no
  visible surface existed to report "no voice installed" without opening the
  Translate Window, which would steal focus and show the wrong feature. Nothing is
  shown for the nothing-selected outcome.
- The non-macOS fallback (`speech_unsupported.rs`) is a typed `Unavailable`, like
  the capture fallback, so the crate still builds away from macOS.

## Verification

All commands run from the repository root.

| Command | Result |
| --- | --- |
| `./scripts/test.sh` | exit 0 — `tsc --noEmit`, `vite build`, the IPC boundary check, and `cargo test --workspace`: 2 shell shortcut-routing tests, 5 language-heuristic tests, 3 dependency-boundary tests, 22 engine-seam tests, 3 settings-store tests |
| `cargo clippy --workspace --all-targets` | exit 0, no warnings |
| `./scripts/build.sh` | exit 0 — packages and signs `target/bundle/Translator.app` with `Translator Local Signing` |
| `codesign -d -r- target/bundle/Translator.app` | `designated => identifier "com.taitran.translator" and certificate root = H"0210dfaace23b661886724d8588f09a5ee30882b"` |
| `otool -L target/bundle/Translator.app/Contents/MacOS/translator \| grep AVFoundation` | the binary links `/System/Library/Frameworks/AVFoundation.framework` |
| `nm -gU target/bundle/Translator.app/Contents/MacOS/translator \| grep _tl_speak` | `T _tl_speak` — the speech bridge is compiled and linked |
| `say -v '?' \| grep -i viet` | `Linh (Vietnamese (Vietnam)) vi_VN` — one Vietnamese voice is installed on this machine |

The engine-seam tests cover the observable contract:

| Checkbox | Tests |
| --- | --- |
| Speaks only the Selection, never a translation | `read_aloud_speaks_the_selection_and_never_the_translation` |
| Nothing selected → no speech, success | `read_aloud_with_nothing_selected_succeeds_without_speaking` |
| No provider call on any Read Aloud path | `read_aloud_speaks_the_selection_and_never_the_translation`, `read_aloud_with_nothing_selected_succeeds_without_speaking` |
| Voice chosen from the language | `read_aloud_picks_the_voice_from_the_language_the_selection_is_written_in`, `read_aloud_falls_back_to_english_when_there_is_no_letter_to_judge_by`, `language::tests::*` |
| A missing voice is a typed failure naming the language | `a_missing_voice_is_a_typed_speech_failure_that_names_the_language` |
| A missing grant and secure input stay typed errors | `read_aloud_reports_a_missing_accessibility_grant_instead_of_reading`, `secure_input_stops_read_aloud_before_anything_is_spoken` |
| `⌃⌥S` is routed to Read Aloud, and nowhere if it did not register | `each_registered_shortcut_routes_to_its_own_action`, `a_shortcut_that_did_not_register_routes_nowhere` |

Runtime behaviour — the voice actually speaking, not blocking the app, stopping a
previous utterance, and the failure dialog — is manual; the steps are in
[`../checks/03-read-aloud-manual.md`](../checks/03-read-aloud-manual.md). Those
cases have **not** been run: this ticket built the app but did not launch it.
