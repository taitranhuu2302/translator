# Windows port

Status: ready-for-agent (spike first; no implementation tickets are open yet)

## Problem Statement

ADR-0001 promises a second platform crate and says `core` will not change when
it arrives. Today nothing has arrived. The shell cannot even be compiled on
Windows: `crates/app` depends unconditionally on `tauri-nspanel` and the
`macos-private-api` feature, `translator-platform-macos` depends on
`security-framework` and compiles AppKit, AVFoundation and Carbon Objective-C,
and `history`, `settings_file` and `migration` have no non-macOS branch. Only
`capture`, `speech`, `credentials`, `clipboard`, `bundle` and `pointer` carry a
`#[cfg(not(target_os = "macos"))]` fallback, and those fallbacks fail by
design.

The product loop, not the API surface, is what has to be rebuilt: read the
text the user selected in another application, translate it, show a popup that
does **not** take focus from the app the user is typing in, replace the
selection in place, speak it, and remember what happened. Every one of those
has a different Windows mechanism than its macOS counterpart, and two of them
carry real risk that no amount of reading settles:

- **UI Automation** is the Windows analogue of the Accessibility API, and how
  much text it exposes differs per application — Chrome, Electron, terminals
  and Office each answer differently. The capture ladder's *shape* (a rich path
  first, a clipboard round trip second, secure fields refused) can be kept, but
  where each app lands on it is a fact about Windows, not a design choice.
- **A non-activating, key-accepting popup** (`WS_EX_NOACTIVATE`, no activation
  on show, still receiving Enter and Escape) is the Windows counterpart of the
  `tauri-nspanel` spike that ADR-0005 records. It is the highest-risk part of
  the port and is proven before the rest is planned, exactly as it was on
  macOS.

## Solution

The port keeps the three-crate shape and adds a fourth:

- `platform-windows` — the only crate that may touch Win32, UI Automation, the
  Credential Manager, SAPI and the registry. It implements the same ports
  `platform-macos` does. `core` is not edited.
- `app` — grows `#[cfg(target_os)]` seams where it names macOS-only crates
  (`tauri-nspanel`, the `macos-private-api` feature, the AppKit panel code) and
  a Windows presenter for the same workflows.

The first ticket is the spike, and its answer decides the rest.

**Slice 0 has landed** in `.scratch/windows-build/`: `tauri-nspanel` is a macOS
target dependency, `panel` and `hud` have non-AppKit counterparts with the same
surface, the LaunchAgent launcher and the activation policy are gated, and
`icons/icon.ico` exists. `cargo build -p translator-app` is gated on every pull
request. The port starts from a shell that compiles, not from one that cannot.
The `macos-private-api` feature and the Windows presenter of the same workflows
are still open.

## CI and release contract

Fixed here so the port plugs into the pipeline instead of reopening it:

| Thing | macOS (shipped) | Windows |
| --- | --- | --- |
| Artifact | `Neris_<ver>_universal.dmg`, `Neris.app.tar.gz` | `Neris_<ver>_x64-setup.exe` |
| Installer | drag to `/Applications` | NSIS, **per-user**, no admin prompt |
| Update key in `latest.json` | `darwin-aarch64`, `darwin-x86_64` | `windows-x86_64` |
| Architectures | universal (arm64 + x86_64) | x86_64 first; arm64 only when someone needs it |
| Signing | self-signed identity, quarantined download | unsigned; SmartScreen "More info → Run anyway" on first install only |
| Minimum OS | macOS 14 | Windows 10 21H2, WebView2 present by default |
| CI job | full suite | `cargo test -p translator-core` until the port compiles, then the workspace |

The update payload is verified by the same minisign key on both platforms, so
an unsigned installer still updates silently; Authenticode is a separate, later
decision that would only remove the first-install SmartScreen warning.

## The menu of work (not yet ticketed)

Each of these becomes a ticket when the spike lands; the order below is the
dependency order.

1. `platform-windows` skeleton and the second capture ladder — UI Automation
   `TextPattern`, clipboard round trip, secure-field refusal.
2. Replace — clipboard plus synthesized paste, restoring the previous clipboard
   contents, reporting every failure.
3. Credentials — Windows Credential Manager, one entry per provider.
4. Speech — SAPI or `Windows.Media.SpeechSynthesis`, stopping the previous
   utterance.
5. Settings, history and migration under `%APPDATA%\Neris`, with the same
   SQLite schema; the rename migration is macOS-only and must not fire.
6. The `app` seams: `tauri-nspanel` behind `cfg`, a non-activating popup that
   still accepts keys, the tray icon as `.ico`, `LSUIElement`'s equivalent (no
   taskbar entry) for the popup and HUD.
7. Global shortcuts and start-at-login (the existing plugins already support
   Windows).
8. NSIS packaging script, the `windows-x86_64` entry in `latest.json`, and the
   release workflow's second job.
9. A Windows manual test plan under `checks/`, walked on a real machine.

## Out of scope

- Linux.
- An MSI (needs WiX and admin rights; NSIS per-user is the update path).
- Authenticode signing at first release.
