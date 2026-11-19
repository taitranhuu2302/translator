# 03 — One name everywhere

Status: done

## What

Replace every user-visible "Translator" with "Neris" across the shell, the engine's diagnostics, the UI copy and the living docs, and record the rule so it does not drift back.

## Where

- `crates/app/src/{main,tray,shortcuts,workflows}.rs` — tray tooltip and Quit item, Accessibility copy, login-item copy, `Neris:` log prefixes
- `crates/core/src/{http,engine}.rs` — `Neris/` User-Agent, history diagnostics
- `ui/src/{Onboarding,SettingsPane,notice,ipc}.tsx` and their tests, `ui/index.html`, `ui/preview.html`
- `README.md`, `GLOSSARY.md`, `DESIGN.md` — titles, the Brand/Product split removed, brand assets documented, `Waves` replaced by `NerisMark`
- `docs/adr/0007-rename-to-neris.md` — new; `0004` notes the one identifier change

## Not renamed

The repository, the remote, the crates, the binary, `CFBundleExecutable`, `TRAY_ID`, `TranslatorEngine`, Azure's "Translator" product name, and "Quick Translate".

## Done when

- No user-visible string says "Translator".
- The remaining `Translator` occurrences are identifiers or Microsoft's product name, and a grep says so.
