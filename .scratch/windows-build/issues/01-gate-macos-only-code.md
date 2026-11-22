# Gate the macOS-only shell code

Status: done

## What

Make `cargo build -p translator-app` succeed on `x86_64-pc-windows-msvc`
without changing macOS behaviour.

Known blockers:

1. `crates/app/Cargo.toml` depends on `tauri-nspanel` unconditionally; the crate
   imports `objc2` types that only exist under `cfg(target_os = "macos")`. Move
   it to `[target.'cfg(target_os = "macos")'.dependencies]`.
2. `crates/app/src/panel.rs` and `hud.rs` are AppKit panels end to end. Give
   each a `#[path]` counterpart for other platforms with the same public API:
   `panel::{POPUP_LABEL, POPUP_PREFILL_EVENT, prepare, show, set_height, hide}`
   and `hud::{prepare, ReplaceHud::{start, phase, phase_now, finish}}`.
3. `main.rs` calls `tauri_nspanel::init()`, `.macos_launcher(...)` and
   `app.set_activation_policy(...)` unconditionally; each needs a `cfg`.
4. `main.rs` builds the tray with `.icon_as_template(true)`, a macOS-only
   builder method.
5. `tauri-build` refuses a Windows target with no `.ico` in `bundle.icon` or
   `icons/icon.ico`. Add `crates/app/icons/icon.ico`, generated from
   `icon.png`.

## Why

A Windows installer cannot be built for a binary that does not compile. This is
slice 0 of `windows-port/spec.md`: the port inherits it.

## Acceptance

- `cargo build -p translator-app` on `windows-latest` exits 0.
- `cargo build -p translator-app --features custom-protocol` on macOS still
  exits 0, and `cargo test -p translator-app` still passes.
- No behaviour change on macOS: the panel, HUD, tray template, activation policy
  and login item are all exactly as before.
- On Windows the app launches and shows its window; the unsupported modules say
  what is missing instead of panicking.

## Result

The five listed blockers were the whole of `crates/app`. The first Windows run
found a sixth, one crate over: `translator-platform-macos` depended on
`security-framework` unconditionally, and while that crate empties itself off
Apple, its dependency `core-foundation` does not — it reaches for `std::os::unix`
and `libc::PATH_MAX`, so it failed the build before the crate's own `cfg`s were
read. Moved to `[target.'cfg(target_os = "macos")'.dependencies]`; no source
change, because every use in `credentials.rs` was already gated. macOS is
untouched: `cargo fmt --check`, `cargo test -p translator-app` (50) and
`./scripts/test.sh` all pass.
