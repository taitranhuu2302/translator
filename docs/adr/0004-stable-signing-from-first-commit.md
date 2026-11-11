# Sign every build with a stable identity

macOS pins the Accessibility permission grant to the code signature, not the bundle ID. An ad-hoc signed build has a designated requirement of `cdhash H"…"`, so every rebuild changes it: the stored grant stops matching and AX calls fail silently with error `-1719` while System Settings still shows the toggle on.

Every build, including development ones, is signed with a stable self-signed certificate in the login keychain, and the bundle identifier never changes (`com.taitran.translator`).

Packaging is part of the build, not a manual step: the build emits a real `.app` with the menu-bar-only `LSUIElement` key and signs it. A Tauri dev run is not signed, which is why the manual checklist runs against the packaged bundle rather than `cargo tauri dev`.

**Consequences:** a one-time certificate setup before the first run, and a signing step in the build script — the project is not buildable unsigned. Without this, a rebuild looks like broken AX code and costs a day of debugging; with it, TCC grants survive rebuilds and a Developer ID can replace the self-signed cert later without disturbing permissions.
