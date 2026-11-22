# Distribute under the local identity, and update in place

Distribution keeps the identity ADR-0004 established: the released app is
signed by the same self-signed certificate in this Mac's login keychain, now
exported to GitHub Actions as a secret. The Apple Developer Program was
considered and deliberately not bought yet. Its cost is the Gatekeeper
consequence below; its benefit is that **the designated requirement never
changes**, so the Accessibility grant survives every update, and the first
released build inherits the grant this Mac already has rather than asking for
a second one.

Two consequences are accepted rather than hidden:

- A build downloaded from GitHub carries the quarantine flag, so the first
  install needs one command — `xattr -dr com.apple.quarantine
  /Applications/Neris.app` — documented in `docs/install.md`, the README and
  every release's notes. An update written by the app itself is not
  quarantined, so this is paid once, not per release.
- The certificate that signs the first release must sign **every** release.
  Changing it changes the designated requirement, and every user loses the
  Accessibility grant once. A Developer ID can replace it later at exactly that
  price; that is the whole trade.

Updates are the Tauri updater plugin reading `latest.json` from the repository's
latest GitHub Release. The payload is a `.app` tarball (a DMG is never
updated), verified by a minisign key whose public half is authored in
`tauri.conf.json` and whose private half lives only in GitHub Secrets. The app
checks on launch and every six hours, downloads in the background, and waits: a
new version adds a menu-bar row and a Settings action, and installing restarts
the app. It never restarts on its own — this is a menu-bar app that may be
mid-translation.

The updater is compiled only into release builds, behind the `updater` cargo
feature. A local `./scripts/build.sh` reports updates as disabled, so the bundle
under development cannot be replaced by a release. One channel, stable only;
the Windows port will add `windows-x86_64` to the same manifest with an
unsigned NSIS installer, which updates silently because the artifact signature
— not Authenticode — is what the updater verifies.

**Consequences:** losing the update private key means no further in-app
updates for existing installs (they can still install manually). Losing the
signing certificate has the same effect plus a re-grant. Both are written down
in `docs/release.md`, and the release script refuses to build with a
placeholder public key.
