# Translator

A macOS menu-bar app for translating text you have selected in any app — without leaving the app you are typing in.

**Status: scaffolding.** The Xcode project builds and runs today: a menu-bar item, four global shortcuts, the Translate Window shell, and the non-activating popup shell. Selection capture, translation providers, Replace, and Read Aloud are specified and ticketed but not implemented yet.

## Shortcuts

| Shortcut | Action |
| --- | --- |
| `⌃⌥D` | **Quick Translate** — popup next to the cursor with the translation |
| `⌃⌥R` | **Replace** the Selection with its translation |
| `⌃⌥S` | **Read Aloud** — speak the Selection in the system voice |
| `⌃⌥E` | Open the **Translate Window** |

## How it works

**Direction.** A Language Pair holds one configured Target. A Selection already in the Target language is translated into the other member of the pair; anything else is translated into the Target.

**Translation.** Azure Translator F0 as the primary provider, with a keyless fallback so the app works before any key is configured.

**Improve.** The Translate Window's second mode rewrites a sentence: the corrected version and a better version, side by side.

**Capture.** The macOS Accessibility API first; a clipboard round trip for apps that expose nothing (browsers, Electron apps, terminals). Every path reports failure rather than silently doing nothing, and secure fields are refused outright.

## Requirements

- macOS 14 or later
- Xcode 26

## Build

```bash
xcodebuild -project Translator.xcodeproj -scheme Translator -configuration Debug -derivedDataPath build build
open build/Build/Products/Debug/Translator.app
```

Development builds are currently signed ad-hoc. A stable self-signed identity is required before granting Accessibility permission — see [ADR-0004](docs/adr/0004-stable-signing-from-first-commit.md).

## Project docs

- [GLOSSARY.md](GLOSSARY.md) — the domain language used in code, specs and tickets
- [docs/adr/](docs/adr/) — architecture decisions
- [.scratch/translator-v1/spec.md](.scratch/translator-v1/spec.md) — the v1 spec
- [.scratch/translator-v1/issues/](.scratch/translator-v1/issues/) — implementation tickets

## License

MIT — see [LICENSE](LICENSE).
