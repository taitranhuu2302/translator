# Windows install check

Run this on the owner's Windows machine, on a normal (non-administrator)
account. Record the result in `issues/04-install-on-a-real-machine.md`.

## Before

- [ ] Windows 10 21H2 or later, 64-bit.
- [ ] No older Neris installed (Apps & Features is clean).
- [ ] The `.exe` came from the workflow run, not from a local build.

## Install

- [ ] Double-clicking the `.exe` does **not** raise a UAC prompt.
- [ ] SmartScreen appears once ("Windows protected your PC"); More info → Run
      anyway starts the installer. This is expected for an unsigned installer.
- [ ] The installer finishes without an error dialog.
- [ ] Start Menu has a Neris entry.
- [ ] Apps & Features lists Neris with the version from `scripts/version.sh`.

## Open

- [ ] Launching from the Start Menu shows the main window.
- [ ] The window has content — a blank white view means WebView2 failed and is
      a failure, not a slow start.
- [ ] The window can be closed and reopened.
- [ ] Settings → provider key can be typed; the failure it reports is the
      typed "macOS only" one, not a crash.

## Not expected to work yet

These are the port, and a failure here is **not** a failure of this check:

- Selection capture, Replace, Read Aloud.
- Quick Translate popup as a non-activating panel.
- The menu-bar/tray item and its template icon.
- Storing settings or history under `%APPDATA%\Neris`.
- Global shortcuts firing while another app is focused.

## Remove

- [ ] Uninstall from Apps & Features completes without an administrator prompt.
- [ ] The Start Menu entry and the installation folder are gone.
- [ ] What is left behind is understood: Tauri's per-user install writes to
      `%LOCALAPPDATA%`; settings under `%APPDATA%\Neris` are expected to remain
      only once the port stores them there.
