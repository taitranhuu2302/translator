# Translator v1

Status: ready-for-agent

## Problem Statement

Translating a sentence while writing an email, a Slack message, or a document means leaving the app, opening a translator, copying the text out, copying the result back, and hoping the formatting survived. It is slow enough that people stop doing it and ship the wrong sentence instead.

The same person is often reading English and wants to hear a word or a sentence pronounced, and often writes English that is understandable but not quite right, with no quick way to get it corrected and improved.

Existing menu-bar translators are either slow to open, visually unpleasant, or depend on a cloud API that requires billing to set up — so the app is useless until the user has an account.

The project itself has a second problem: it is written in Swift against AppKit, so every piece of product logic — direction resolution, the provider ladder, chunking, history, settings — is locked to macOS and can only be exercised by building the app. Porting later means rewriting that logic, and testing it today means launching Xcode.

## Solution

A macOS menu-bar app that acts on the Selection in whatever app is focused, through four global shortcuts:

- **Quick Translate** (`⌃⌥D`) reads the Selection, shows a small popup next to the cursor with the translation, and lets the user press Enter to Replace the Selection.
- **Replace** (`⌃⌥R`) translates and Replaces in one step, with no confirmation.
- **Read Aloud** (`⌃⌥S`) speaks the Selection in the system voice. It never speaks a translation.
- **Translate Window** (`⌃⌥E`) opens a two-pane window with the text on one side and the translation or improvement on the other, with a `Translate | Improve` switch.

Direction is inferred from the Language Pair: if the Selection is already in the Target language, it is translated into the other member; otherwise into the Target.

Translation comes from Azure Translator F0, falling back to a keyless provider so the app works before any key is configured. Improve uses Groq. The app lives only in the menu bar, opens fast enough that a shortcut never feels like waiting for a program to start, and never steals focus from the app the user is typing in.

Underneath, the product logic moves into a Rust core with no platform dependency. The macOS layer is one implementation of that core's ports, and the UI is a web view. A second platform can later add an implementation without touching translation, direction, chunking, caching, history, or settings.

## User Stories

1. As someone writing an email, I want to select a Vietnamese sentence and press `⌃⌥D`, so that I see the English translation beside my cursor without leaving the compose window.
2. As someone who just selected text, I want the popup to appear without taking focus, so that my caret stays exactly where I was typing.
3. As someone happy with the translation, I want to press Enter to Replace the Selection, so that I never touch the clipboard myself.
4. As someone in a hurry, I want `⌃⌥R` to translate and Replace in one step, so that I skip the confirmation.
5. As someone who selected English text while my Target is English, I want the app to translate into the other member of the pair, so that I don't have to flip a setting.
6. As someone reading an English article, I want to select a word and press `⌃⌥S`, so that I hear it pronounced.
7. As someone learning English, I want Read Aloud to speak only the original Selection and never the translation, so that I hear the pronunciation I am trying to learn.
8. As someone using Chrome, VS Code, or Terminal, I want Quick Translate to still work when the app does not expose its Selection through Accessibility, so that the shortcut is not silently broken in half my apps.
9. As someone whose text had to be read through the clipboard, I want my previous clipboard contents restored, so that I don't lose what I had copied.
10. As someone in an app that refuses text insertion, I want Replace to fail visibly, so that I know to paste manually instead of believing it worked.
11. As someone with a password field focused, I want the app to refuse to read the Selection, so that a secret is never captured, logged, or sent to a provider.
12. As someone who has not granted Accessibility, I want a plain explanation of why it is needed and a button to open the right settings pane, so that I can fix it in one step.
13. As someone on a slow connection, I want the popup to open immediately and fill in when the translation arrives, so that I know the shortcut registered.
14. As someone who has not configured any key, I want the app to fall back to a keyless provider, so that it works on first launch.
15. As someone whose provider quota ran out, I want an error that names the provider and suggests the next step, so that I know what to fix.
16. As someone translating a long paragraph, I want it split into chunks transparently, so that long text still translates instead of failing.
17. As someone who translates the same sentence twice, I want the second lookup to be instant, so that I don't spend quota on a repeat.
18. As someone who pressed the wrong shortcut, I want Escape to dismiss the popup without changing anything, so that a mistake costs nothing.
19. As someone translating more than a sentence, I want `⌃⌥E` to open the Translate Window, so that I get a real editor instead of a popup.
20. As someone comparing languages, I want the Translate Window split into two panes, so that the source and the result sit side by side.
21. As someone writing English, I want an Improve mode, so that I get both a corrected version and a better version of my sentence.
22. As someone using Improve, I want the corrected and improved versions clearly separated, so that I can copy the one I want.
23. As someone working keyboard-only, I want Enter to translate and `⌘C` to copy the result, so that I never reach for the mouse.
24. As someone waiting on the model, I want Improve to stream, so that I see the first words immediately instead of a spinner.
25. As someone who translates every day, I want my Language Pair remembered between launches, so that I configure it once.
26. As someone who set Target to English, I want the other language to be implied, so that the pair is one setting rather than two.
27. As someone who translated something last week, I want to search local history for 30 days, so that I can find an earlier translation.
28. As someone privacy-conscious, I want history to stay on this Mac and age out automatically, so that old text does not accumulate forever.
29. As someone with API keys, I want them stored in the Keychain, so that they never sit in a plain file I might sync or share.
30. As someone launching the app, I want it to open as fast as the machine allows and to start with my Mac if I choose, so that a shortcut never feels like waiting for a program to start.
31. As someone who runs it all day, I want it in the menu bar only, so that it does not clutter the Dock or `⌘Tab`.
32. As someone with several desktops, I want the popup on the active Space, so that it appears where I am working.
33. As someone inside a full-screen app, I want shortcuts and the popup to work, so that I don't have to leave full screen.
34. As someone translating inside the Translate Window itself, I want the same shortcuts to act on my own Selection, so that behaviour is consistent everywhere.
35. As someone whose shortcut collides with another app, I want to change the key combination, so that I can resolve it myself.
36. As someone reading a translation, I want to select part of the result with the mouse, so that I can copy a phrase rather than the whole thing.
37. As someone translating both directions in one session, I want a swap control in the Translate Window, so that I can reverse one translation without changing Target.
38. As someone offline, I want a specific offline error rather than a generic failure, so that I don't assume the app is broken.
39. As someone trying the app for the first time, I want a short onboarding that grants Accessibility and lists the four shortcuts, so that I know how to begin.
40. As someone who rebuilds or updates the app, I want the Accessibility grant to survive, so that capture does not silently stop working after every release.
41. As someone pressing the shortcut with nothing selected, I want to be told to select something, so that the app does not look broken.
42. As someone whose primary provider failed, I want the translation to still arrive from the fallback and to say which provider answered, so that I can trust the result.
43. As someone who wants the result without Replace, I want to copy the translation from the popup, so that I can paste it somewhere else.
44. As someone who speaks a language other than Vietnamese or English, I want to choose another pair from the languages the providers support, so that the app is not limited to one pair.
45. As someone who never opens the Translate Window, I want the menu bar item to show the shortcuts and the permission state, so that I can use the app without hunting for a settings screen.
46. As someone who changes my mind, I want to turn off start-at-login and see the app already running, so that I control when it launches.
47. As someone who cares about what leaves my Mac, I want only the text I am translating to be sent to a provider, so that history, settings and other text stay local.

## Implementation Decisions

**Shape of v1.** A menu-bar-only macOS app (`LSUIElement`, accessory activation policy) built as a Rust workspace plus a web UI. Three parts:

- **`core`** — no platform dependency at all. Owns the `TranslatorEngine`, direction resolution, sentence chunking, the provider ladder, the LRU cache, history, settings, and the port traits the platform must satisfy: selection capture, translation provider, speech output, history store, settings store, and credential store. Everything that has a rule worth testing lives here.
- **`platform-macos`** — implements the ports: Accessibility and clipboard capture, synthesized key events, Carbon global hotkeys, Keychain, SQLite, speech synthesis, and the non-activating panel.
- **`app`** — the Tauri v2 shell: builds the ports, owns the engine, exposes IPC, and hosts the React UI.

**One seam.** Tests drive `TranslatorEngine` through its public API with fake ports and assert on observable outcomes: the translation returned, the typed error surfaced, the order in which ports were called, the bytes sent to a provider, what was written to history. No test asserts on a private helper, on which internal function ran, or on view structure. The frontend has exactly one equivalent seam: the IPC client module. Components are tested through it, never by reaching into Tauri directly.

**Engine contract.** The engine exposes four operations — quick translate a Selection, replace a Selection, translate text with a streaming event stream, and improve text with the same event stream shape. Every outcome is a typed success or a typed failure; there is no "silently did nothing" path. Streaming emits the same event shape for machine translation and model output so the views have one code path.

**Capture and Replace.** Accessibility first, clipboard as fallback. Read: the focused element's selected-text attribute, then a synthesized copy with the clipboard saved, read, and restored. Write: the selected-text attribute only when it reports itself settable, otherwise the clipboard plus a synthesized paste, with the previous clipboard restored. The Accessibility attribute names are not generated by the Rust bindings and must be built from their string values. Clipboard restoration must compare the pasteboard change count before overwriting, because a naive save/restore is how clipboard-pollution bugs ship. Every path reports a typed failure. Secure event input is refused outright before any read.

**The popup.** Quick Translate is a Tauri window converted into a non-activating panel: it may become key so that Enter and Escape reach the web view, but it must not activate the app, and it must float above normal windows, join every Space, and appear over a full-screen app. This is the highest-risk part of the design and is proven by a spike before the architecture is treated as settled. If the combination cannot be made to work, the fallback is a native popup drawing the translation without a web view, with the Translate Window remaining React.

**Startup.** The app opens as fast as the platform allows; there is no fixed millisecond budget in the spec, and none in the tests. The app pre-creates its windows hidden at launch and offers a start-at-login preference, so the expensive first start is paid once rather than on every shortcut. The menu-bar item and shortcuts must be registered before any window is shown.

**IPC.** One thin client module on the frontend wraps every Tauri command with hand-written types; no component calls the Tauri API directly. Types may be generated from Rust later if drift actually appears, but v1 does not take the dependency. Commands carry the engine's operations and settings/history queries; events carry translation chunks, completion, errors, and provider status.

**Provider ladder.** Azure Translator F0 primary, MyMemory as the keyless fallback, Groq for Improve and for text the others cannot handle. Providers are tried in order; a provider that fails with a quota or authentication error is skipped for the rest of the session, and the response records which provider answered. The provider interface is a port so the ladder can change without touching the UI or the engine's callers.

**Chunking and caching.** MyMemory caps a request at 500 bytes and Vietnamese diacritics cost two to three bytes per character, so text is split on sentence boundaries into chunks that fit the budget, translated, and rejoined in order without loss. Both providers sit behind an in-memory LRU keyed by text and direction, so a repeated Selection never spends quota.

**Direction resolution.** The Language Pair holds exactly one configured Target and one implied partner. A Selection already in the Target language is translated into the partner; anything else into the Target. Detection uses the provider's reported language when available and a lightweight local heuristic before the request.

**Improve contract.** Improve asks for exactly two labelled parts: a corrected sentence that preserves the author's meaning and register, and a better sentence that is more natural or concise. Both are returned as separate fields, never as prose the view has to parse, and malformed model output is a typed error rather than a blank pane.

**Storage.** API keys in the Keychain, one item per provider. History in SQLite under Application Support, holding both sides of every translation, the provider, the mode, and a timestamp; rows older than 30 days are purged at launch and once a day. The Language Pair, shortcut bindings and the start-at-login preference live in a local settings file that is not synced.

**Shortcuts.** `⌃⌥D`, `⌃⌥R`, `⌃⌥S`, `⌃⌥E`, registered through Carbon so that no Input Monitoring permission is needed. Bindings are user-editable and persisted; a binding that cannot be registered surfaces a conflict rather than failing silently.

**UI.** React with TypeScript, Vite, Tailwind, and shadcn components for the Translate Window, settings, and onboarding. The popup uses the same design tokens but is deliberately built from minimal primitives: a dialog stack that traps focus is useful there, but nothing in the popup may depend on behaviour that activates the app. The visual language previously expressed in SwiftUI is re-created in Tailwind rather than carried over component by component.

**Signing and identity.** The bundle identifier stays `com.taitran.translator`, and every build is signed with a stable identity. macOS pins the Accessibility grant to the signature, so an ad-hoc signed build loses the grant on every rebuild while System Settings still shows the toggle enabled. The build produces a real `.app` bundle with the menu-bar-only plist key; packaging is part of the build, not a manual step.

**Target platform.** macOS 14 or later for v1. The `core` crate is where the cross-platform promise is kept: it has no AppKit, no Tauri, and no macOS-specific dependency. A Windows milestone later adds a second platform crate and a second capture ladder; nothing in `core` is abstracted further than that today.

**Documentation.** The Swift-era architecture records and the Swift sources are removed. New records describe the Rust core, the Tauri shell, the non-activating panel and the signing rule; the still-valid reasoning about Accessibility-first capture and the provider ladder is carried into them rather than lost. The glossary is unchanged, because the product's vocabulary did not change. This spec replaces the previous v1 spec in the same directory; the tickets are rewritten where they name Swift or AppKit and kept where they describe behaviour.

**Contradiction to record.** This reverses the earlier decision to build the app in Swift; that record is deleted and replaced by ADR-0001 ("Rust core with a Tauri shell"). The constraints it identified — the popup must not steal focus from the app being typed in, and startup must not feel slow — are not dropped; they become requirements on a different implementation, and the popup one is proven by the spike.

## Testing Decisions

**What a good test is here.** A test drives the engine through its public API with fake ports and asserts on observable outcomes: the translation returned, the typed error, the order of port calls, the bytes sent to a provider, what was written to history. Tests never assert on private helpers, view structure, or internal call counts that do not change behaviour.

**Seam.** One seam in Rust — the engine — with fakes only for the ports, kept in a single shared test-support module: a capture port that can be told to succeed, fail, or require the clipboard path; a provider that returns a scripted response or throws a scripted error; a recording speech port; an in-memory history store; an in-memory settings store; a fake credential store. One seam on the frontend — the IPC client — faked with scripted events. No UI component is tested through Tauri directly.

**Modules under test.** Direction resolution including the same-language case; provider ladder fallback and the session-wide skip after a quota error; chunking of Vietnamese text against the byte budget and lossless rejoin; the improve round trip including malformed model output; history recording and purge; Replace gating as observed through the fake capture port; cache hit behaviour including direction in the key; settings round trip.

**Frontend testing.** Only logic worth testing is tested: direction and error formatting, popup state transitions (opening, filled, error, dismissed), history search filtering. Components are not snapshot-tested.

**Not unit-tested.** Real Accessibility behaviour, the synthesized copy and paste round trip, TCC prompts, panel focus, Space and full-screen behaviour, and start-at-login. These are verified by a manual checklist against the built app, covering at minimum: Gmail in Chrome, Safari page text, VS Code, Terminal, Notes, a password field, a full-screen app, and a second Space.

**Prior art.** None — this is the project's first test target. One file per behaviour area, with the fakes in a single support module so the port shapes stay in one place.

## Out of Scope

- Windows and Linux in v1. The core is portable; no second platform ships yet.
- App Store distribution, notarization, and auto-update.
- On-device or offline models; the keyless fallback still needs a network.
- OCR or screenshot translation.
- Syncing history or settings between Macs.
- User-managed glossaries or translation memory beyond the LRU cache.
- Gemini as a last-resort provider; it stays documented but unwired.
- Google Cloud Translation, which needs billing before the first request.
- A preferences window beyond the minimum needed to set the pair, keys, shortcuts and start-at-login.
- An end-to-end UI test suite for the desktop app.

## Further Notes

The popup spike comes first and is the gate for everything else: one global shortcut, read the Selection, show a non-activating panel over a full-screen app without taking focus, press Enter, and watch the Replace land in the app that had focus. If that works, the architecture is settled. If it does not, the popup becomes native and the rest of the plan is unchanged.

It passed on 2026-11-11: the panel shows without activating the app, the web view receives keys, Enter and Escape work, typing returns to the host app when the panel closes, and the panel is on the active Space over a full-screen app. The one trap found — the web view must be made first responder, not its container view — is recorded in ADR-0005.

The build is Rust plus a pre-built frontend; Node is a build-time dependency only, never a runtime one. Packaging must emit a menu-bar-only bundle and sign it with the stable identity, or every rebuild silently loses the Accessibility grant.

Read Aloud uses the system speech synthesizer. macOS ships exactly one Vietnamese voice and its quality is poor, so Read Aloud is expected to be used mostly on English, which is why the feature is scoped to the Selection and not to translations.

Some apps never expose a settable selected-text attribute — browsers, Electron apps, and terminals among them — so the clipboard path is the normal path there, not an exception. Its correctness (restoring the clipboard, not racing the paste) matters more than the Accessibility path's.
