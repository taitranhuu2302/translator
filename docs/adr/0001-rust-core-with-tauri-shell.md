# Rust core with a Tauri shell

The product logic is not macOS-specific. Direction resolution, the provider ladder, sentence chunking, the cache, history and settings are rules about text, and none of them needs AppKit. Writing them in Swift locks every one of those rules to one platform and to Xcode, so testing a chunking rule means building the app.

The stack is a Rust workspace with a web view on top:

- `core` — no platform dependency. Owns the engine, the rules above, and the port traits the platform must satisfy.
- `platform-macos` — the only crate that may touch AppKit, ApplicationServices, AVFoundation or the Keychain.
- `app` — a Tauri v2 shell that builds the ports, owns the engine and hosts a React UI.

macOS ships first; a Windows milestone later adds a second platform crate and leaves `core` alone.

This reverses the earlier decision to build the app in Swift. The constraint that produced that decision was answered first rather than argued away: the popup must accept typing without activating the app. That was verified against `tauri-nspanel` 2.1.1 before the rest of the plan was treated as settled — see ADR-0005.

**Consequences:** startup is slower than a native accessory app, so windows are pre-created hidden at launch and start-at-login is offered; a shortcut never pays for a first start. There are two runtimes to reason about — Rust and the system web view — and Node is a build-time dependency that must never become a runtime one. Objective-C is still required for Accessibility, Carbon hotkeys and the panel, but it is contained in `platform-macos`. In exchange, every rule with a test lives in `core` and runs under `cargo test` without launching the app.
