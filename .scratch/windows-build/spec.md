# Windows build and installer

Status: in progress — tickets 01–03 are done and a run has produced the
installer; 04 (install it on a real machine) is `ready-for-human`. Decided in a
grilling session on 2026-11-22; slice 0 of the port, see `Blocked by` in each
ticket.

## Problem Statement

`.scratch/windows-port/spec.md` describes the Windows port, and the CI job in
`.github/workflows/ci.yml` proves `translator-core` and the UI are portable. But
a Windows *artifact* cannot be produced at all today: `crates/app` does not
compile off macOS, so there is nothing for an installer to wrap.

This effort is the build half of that gap, not the port itself. It makes the
shell compile on Windows, gives the NSIS bundler the configuration it needs, and
adds a CI job that produces `Neris_<version>_x64-setup.exe` as a workflow
artifact. It does **not** make the app work on Windows: capture, speech,
clipboard, credentials and the panel behaviour stay macOS-only, and the app says
so instead of pretending otherwise.

The dependency is one-way: the port inherits slice 0 from here, and this effort
never waits on the port.

## Decisions

These were decided with the owner; the alternatives are recorded because the
reason is part of the contract.

| # | Decision | Why |
|---|---|---|
| 1 | Scope is **build + installer**. A `v*` tag builds it inside `release.yml` as a job that `needs:` the draft, and attaches `Neris_<ver>_x64-setup.exe` to that draft automatically; `workflow_dispatch` on `windows.yml` still produces a bare artifact for someone to try. `latest.json` gains no `windows-x86_64` key and no updater payload is produced. | Publishing an unsigned installer from a pipeline nobody had used yet was not the first move, so `v0.2.0` attached by hand after two green runs. Once that worked, leaving it manual only guaranteed the file would be missing from the next release. Building it inside `release.yml` rather than in `windows.yml` is what removes the race: the release exists before the job starts. |
| 2 | The Windows job proves **compilation, the portable tests and the installer**, not runtime correctness. | Runtime correctness is the port (`windows-port/spec.md`). Mixing them would hide which half failed. |
| 3 | The installer is **unsigned**. No Authenticode, no certificate, no secret. | The spec already records Authenticode as a later decision; the updater's minisign signature, not Authenticode, is what verifies an update. |
| 4 | **Slice 0 is in scope**: `crates/app` is made to compile on Windows by gating what is macOS-only and adding `icons/icon.ico`. | Without it no installer can exist. It is written down as the port's first slice so the port does not repeat it. |
| 5 | WebView2 uses **`downloadBootstrapper`**. | Windows 10 21H2 and later ship WebView2 with Edge, so the bootstrapper is a safety net; skipping it would leave a blank window on a machine without the runtime. |
| 6 | macOS packaging stays hand-rolled (ADR-0010). Windows gets a **platform config file** `crates/app/tauri.windows.conf.json` and `npx tauri build --bundles nsis`; `tauri.conf.json` is untouched. | Tauri deep-merges the platform file only on Windows, so the macOS path keeps its lipo/PlistBuddy/codesign pipeline with no changes. |
| 7 | Every push and pull request runs **`cargo build -p translator-app`** on `windows-latest`. The installer job runs on **`workflow_dispatch` and tag pushes** only. | A compile break must be visible in the PR that caused it. Packaging a release binary on every PR only spends runner minutes nobody asked for. |
| 8 | The Windows build does **not** enable the `updater` feature. | `latest.json` has no Windows entry, so the feature would only surface a check that cannot succeed. The port turns it on together with the manifest key. |

## What exists when this is done

- `cargo build -p translator-app` succeeds on `windows-latest` with no
  `#[cfg(not(target_os = "macos"))]` shims beyond the two unsupported modules.
- `.github/actions/build-windows-installer` is the one recipe for the `.exe`.
  `windows.yml` runs it on a manual dispatch and leaves the file as an
  artifact; `release.yml` runs it on a tag and attaches it to the draft, so
  every release carries macOS and Windows from the same tag with no hand step.
- The installer is per-user (no UAC prompt), 64-bit, and installing, launching
  and uninstalling it is verified by hand on a real Windows machine — see
  `checks/windows-install.md`.
- A short section in `README.md` and `docs/release.md` says what the artifact is
  and, just as important, what it cannot do yet.

## Not in this effort

- The port: clipboard, selection capture, speech, credentials, global shortcut
  behaviour, tray icon, `%APPDATA%` storage, a Windows `.ico` for the tray.
- MSI/WiX, Authenticode, Linux.
- Attaching the installer to a GitHub release, a `windows-x86_64` entry in
  `latest.json`, or any updater signing for Windows.

## Anatomy

The pieces, in the order the tickets do them:

1. `crates/app/src/main.rs` gates the three macOS-only call sites and chooses
   between `panel`/`hud` and their unsupported counterparts by `cfg`.
2. `crates/app/src/panel_unsupported.rs` and `hud_unsupported.rs` keep the same
   public API so no call site needs a `cfg`.
3. `crates/app/tauri.windows.conf.json` turns on bundling, NSIS, the bootstrapper
   and the `.ico`.
4. `.github/workflows/ci.yml` gains the compile gate;
   `.github/workflows/windows.yml` owns the installer job.
