# Translator v1

Status: ready-for-agent

## Problem Statement

Translating a sentence while writing an email, a Slack message, or a document means leaving the app, opening a translator, copying the text out, copying the result back, and hoping the formatting survived. It is slow enough that people stop doing it and ship the wrong sentence instead.

The same person is often reading English and wants to hear a word or a sentence pronounced, and often writes English that is understandable but not quite right, with no quick way to get it corrected and improved.

Existing menu-bar translators are either slow to open, visually unpleasant, or depend on a cloud API that requires billing to set up — so the app is useless until the user has an account.

## Solution

A macOS menu-bar app that acts on the Selection in whatever app is focused, through four global shortcuts:

- **Quick Translate** (`⌃⌥D`) reads the Selection, shows a small popup next to the cursor with the translation, and lets the user press Enter to Replace the Selection.
- **Replace** (`⌃⌥R`) translates and Replaces in one step, with no confirmation.
- **Read Aloud** (`⌃⌥S`) speaks the Selection in the system voice. It never speaks a translation.
- **Translate Window** (`⌃⌥E`) opens a two-pane window with the text on one side and the translation or improvement on the other, with a `Translate | Improve` switch.

Direction is inferred from the Language Pair: if the Selection is already in the Target language, it is translated into the other member; otherwise into the Target.

Translation comes from Azure Translator F0, falling back to a keyless provider so the app works before any key is configured. Improve uses Groq. The app starts in well under a second, lives only in the menu bar, and never steals focus from the app the user is typing in.

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
29. As someone with API keys, I want them stored in the Keychain, so that they never sit in a plain plist I might sync or share.
30. As someone launching the app, I want it ready in well under a second, so that a shortcut never feels like waiting for a program to start.
31. As someone who runs it all day, I want it in the menu bar only, so that it does not clutter the Dock or `⌘Tab`.
32. As someone with several desktops, I want the popup on the active Space, so that it appears where I am working.
33. As someone inside a full-screen app, I want shortcuts and the popup to work, so that I don't have to leave full screen.
34. As someone translating inside the Translate Window itself, I want the same shortcuts to act on my own Selection, so that behaviour is consistent everywhere.
35. As someone whose shortcut collides with another app, I want to change the key combination, so that I can resolve it myself.
36. As someone reading a translation, I want to select part of the result with the mouse, so that I can copy a phrase rather than the whole thing.
37. As someone translating both directions in one session, I want a swap control in the Translate Window, so that I can reverse one translation without changing Target.
38. As someone offline, I want a specific offline error rather than a generic failure, so that I don't assume the app is broken.
39. As someone trying the app for the first time, I want a short onboarding that grants Accessibility and lists the four shortcuts, so that I know how to begin.

## Implementation Decisions

**Shape of v1.** Menu-bar-only app (`LSUIElement`, `.accessory` activation policy), Swift 6 toolchain in Swift 5 language mode, SwiftUI for view content hosted inside AppKit windows and panels. The existing scaffold already provides the status item, the four Carbon hotkeys, the Translate Window shell, and the non-activating popup shell.

**Capture and Replace.** Accessibility first, clipboard as fallback, per ADR-0002. Reading: try the focused element's selected-text attribute; if the element exposes nothing, save the clipboard, synthesize `⌘C`, read, restore. Writing: Replace only when the selected-text attribute reports itself settable; otherwise write the clipboard and synthesize `⌘V`, then restore. Every path reports a typed failure rather than silently doing nothing. Refuse entirely when secure event input is active (password fields, screen-sharing prompts).

**Provider ladder.** Per ADR-0003: Azure Translator F0 as primary, MyMemory as the keyless fallback, Groq for Improve and for text the others cannot handle. Google's unofficial `translate_a/single` endpoint is not used. Providers are tried in order, and a provider that fails with a quota or auth error is skipped for the remainder of the session.

**Chunking and caching.** MyMemory caps a request at 500 bytes, and Vietnamese diacritics cost 2–3 bytes per character. Text is split on sentence boundaries into chunks that fit the byte budget, translated, and rejoined in order. Both providers are fronted by an in-memory LRU keyed by (text, direction, provider-agnostic), so a repeated Selection does not spend quota.

**Direction resolution.** The Language Pair holds exactly one configured Target and one implied partner. If the Selection looks like the Target language, translate into the partner; otherwise into the Target. Detection uses the provider's detected language when available and a lightweight heuristic (diacritics, stop-words) before the request.

**The one test seam.** All non-UI behaviour lives behind a single engine. Views, panels, and the status item call only the engine; the engine receives its collaborators as ports:

```swift
protocol TranslatorEngine {
    func quickTranslate(_ selection: Selection) async -> QuickTranslateOutcome
    func replace(_ selection: Selection) async -> ReplaceOutcome
    func translate(_ text: String, mode: Mode) -> AsyncStream<TranslationEvent>
    func improve(_ text: String) -> AsyncStream<TranslationEvent>
}

protocol SelectionAccess {           // Accessibility + clipboard, per ADR-0002
    func read() throws -> Selection
    func replace(_ selection: Selection, with text: String) throws
}

protocol TranslationProvider {       // Azure, MyMemory, Groq
    var name: String { get }
    func translate(_ text: String, from: String?, to: String) async throws -> String
}

protocol SpeechOutput { func speak(_ text: String, language: String?) }
protocol HistoryStore { func record(_ entry: HistoryEntry); func search(_ q: String) -> [HistoryEntry] }
```

Ports are injection points, not separate test seams: tests drive the engine and assert on what came back, never on which private helper ran.

**Streaming.** Machine translation returns a single result. Improve streams tokens over SSE from Groq; the engine emits the same event stream shape for both so the views have one code path.

**Improve contract.** Improve asks for exactly two labelled parts — a corrected sentence that preserves the author's meaning and register, and a better sentence that is more natural or concise. Both are returned as separate fields, not as prose the view has to parse.

**Storage.** API keys in the Keychain, one item per provider. History in SQLite in Application Support, holding both sides of every translation, the provider, the mode, and a timestamp; rows older than 30 days are purged at launch and again daily. The Language Pair and shortcut bindings live in user defaults.

**Shortcuts.** `⌃⌥D`, `⌃⌥R`, `⌃⌥S`, `⌃⌥E`, registered through Carbon so no Input Monitoring permission is needed. Bindings are user-editable and persisted; a binding that cannot be registered surfaces a conflict rather than failing silently.

**Window and popup.** The popup is an `NSPanel` that cannot become key or main, floats above normal windows, joins all Spaces, and works in full screen. The Translate Window is a normal window that remembers its frame.

**Language coverage.** The Language Pair offers the languages both primary providers support; v1 ships with Vietnamese and English configured, with the rest selectable but untested.

## Testing Decisions

**What a good test is here.** A test drives the engine through its public API with fake ports and asserts on observable outcomes: the translation returned, the error surfaced, the order in which ports were called, the bytes sent to a provider, what was written to history. Tests never assert on private helpers, view structure, or the number of internal calls that do not change behaviour.

**Seam.** One — the engine. Fakes exist only for the four ports and live in one shared test-support file: an in-memory `SelectionAccess` that can be told to succeed, fail, or require the clipboard path; a `TranslationProvider` that returns a scripted response or throws a scripted error; a recording `SpeechOutput`; an in-memory `HistoryStore`.

**Modules under test.** Direction resolution including the same-language case; provider ladder fallback and the session-wide skip after a quota error; chunking of Vietnamese text against the 500-byte budget and lossless rejoin; the improve prompt round trip including malformed model output; history recording and purge; Replace gating (settable attribute versus clipboard path) as observed through the fake port.

**Not unit-tested.** Real Accessibility behaviour, the synthesized `⌘C`/`⌘V` round trip, TCC prompts, and popup focus behaviour. These are verified by a manual checklist run against the built app, covering at minimum: Gmail in Chrome, Safari page text, VS Code, Terminal, Notes, a password field, and a full-screen app.

**Prior art.** None — v1 introduces the first test target. Use Swift Testing, one file per behaviour area, and keep the fakes in a single support file so the port shapes stay in one place.

## Out of Scope

- Anything other than macOS.
- App Store distribution, notarization, and auto-update (Sparkle comes later).
- On-device or offline models; the keyless fallback still needs a network.
- OCR or screenshot translation.
- Syncing history or settings between Macs.
- User-managed glossaries or translation memory beyond the LRU cache.
- Gemini is documented in ADR-0003 as the last resort but is not wired in v1.
- Google Cloud Translation v2/v3, which needs billing before the first request.
- A preferences window beyond the minimum needed to set the pair, keys, and shortcuts.

## Further Notes

The current scaffold builds with `xcodebuild` and runs, but it is signed ad-hoc. Per ADR-0004, a stable self-signed code-signing identity must be in place before the first Accessibility grant, otherwise TCC pins permission to a `cdhash` that every rebuild changes and capture starts failing with `-1719` while the privacy toggle still looks enabled.

Read Aloud uses `AVSpeechSynthesizer`. macOS ships exactly one Vietnamese voice ("Linh") and its quality is poor; Read Aloud is expected to be used mostly on English, which is why the feature is scoped to the Selection and not to translations.

Some apps never expose a settable selected-text attribute — browsers, Electron apps, and terminals among them — so the clipboard path is the normal path there, not an exception. Its correctness (restoring the clipboard, not racing the paste) matters more than the AX path's.
