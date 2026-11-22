# Install, open and remove it on a real machine

Status: ready-for-human
Blocked by: 03

## What

Download `Neris_<version>_x64-setup.exe` from the workflow run and walk
`checks/windows-install.md` on the owner's Windows machine. Record the result
at the bottom of this file.

## Why

CI proves the installer was produced, not that it installs. The owner has a
Windows machine (settled in the same grilling session), and this is the only
check that can tell "the pipeline ran" from "the artifact is usable".

## Acceptance

- The installer runs without a UAC prompt and without asking for an
  administrator password.
- Neris appears in Start Menu and in Apps & Features, launches, and shows its
  window.
- The window does not show a blank webview (the bootstrapper found or fetched
  WebView2).
- Uninstalling removes the app and leaves settings behind only if that is what
  the checklist says it should do.
- Anything that fails is written down here; it is not silently repaired by
  widening this effort into the port.

## Result

_Not run yet._
