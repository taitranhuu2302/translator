# Give the Windows bundler its configuration

Status: done
Blocked by: 01

## What

Add `crates/app/tauri.windows.conf.json`, merged by Tauri only on Windows:

- `bundle.active: true` and `bundle.targets: ["nsis"]`
- `bundle.icon` listing `icons/icon.png` and `icons/icon.ico`
- `bundle.windows.webviewInstallMode: { "type": "downloadBootstrapper" }`
- `bundle.windows.nsis.installMode: "currentUser"`

`crates/app/tauri.conf.json` keeps `bundle.active: false`; the macOS packaging
script does not go through the Tauri bundler at all (ADR-0010).

## Why

`bundle.active` is false because macOS is packaged by hand. The platform file
lets Windows use Tauri's NSIS bundler — which already implements per-user
install, the WebView2 bootstrapper and uninstall entries — without touching the
macOS path.

## Acceptance

- On a Windows runner, `npx tauri build --config crates/app/tauri.conf.json
  --bundles nsis` writes `target/release/bundle/nsis/Neris_<version>_x64-setup.exe`.
- The installer's name matches `.scratch/windows-port/spec.md`
  (`Neris_<ver>_x64-setup.exe`) with no extra configuration.
- Nothing under `crates/app/tauri.conf.json` changes.
