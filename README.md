# Translator

A macOS menu-bar app for translating text you have selected in any app — without leaving the app you are typing in.

**Status: in progress.** The Rust workspace, the signed bundle and the engine seam exist (ticket 01), and the app now reads the Selection and opens the Translate Window with it (ticket 02). Reading Aloud, Quick Translate, Replace, providers, Improve and History are still stubs behind the same engine seam. The riskiest assumption is proven separately: a Tauri window becomes a non-activating `NSPanel` that receives keys while the app behind it keeps focus (`spike/panel`).

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

## Stack

A Rust workspace with a Tauri v2 shell and a React UI:

- `core` — the product logic and the port traits, with no platform dependency.
- `platform-macos` — the only crate that touches AppKit, ApplicationServices, AVFoundation or the Keychain.
- `app` — the Tauri shell that builds the ports, owns the engine and hosts the UI.

See [ADR-0001](docs/adr/0001-rust-core-with-tauri-shell.md) for the stack and [ADR-0005](docs/adr/0005-non-activating-panel-popup.md) for the popup.

## Requirements

- macOS 14 or later
- Rust toolchain (stable)
- Node 20+ — build-time only, never a runtime dependency

## Build

```bash
./scripts/test.sh     # frontend typecheck and build, IPC boundary check, cargo test
./scripts/build.sh    # build the frontend and the app, then sign target/bundle/Translator.app
```

`scripts/build.sh` packages a real `.app` with `LSUIElement` and signs it with the stable local identity; it refuses to package unsigned or ad-hoc. See [ADR-0004](docs/adr/0004-stable-signing-from-first-commit.md).

The panel spike (deleted when the real popup lands in ticket 04) is still runnable:

```bash
cargo run --manifest-path spike/panel/Cargo.toml
```

Press `⌃⌥D` to show the panel; `Enter` or `Escape` hides it; `⌃C` quits. No bundle, signing or permission is needed for the spike.

## Project docs

- [GLOSSARY.md](GLOSSARY.md) — the domain language used in code, specs and tickets
- [docs/adr/](docs/adr/) — architecture decisions
- [.scratch/translator-v1/spec.md](.scratch/translator-v1/spec.md) — the v1 spec
- [.scratch/translator-v1/issues/](.scratch/translator-v1/issues/) — implementation tickets

## License

MIT — see [LICENSE](LICENSE).
