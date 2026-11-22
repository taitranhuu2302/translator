# Release pipeline and in-app updates

Status: in-progress

## Problem Statement

Neris is feature-complete for one Mac. It is built by `scripts/build.sh`, which
assembles and signs `target/bundle/Neris.app` on the machine that owns the
signing certificate, and the README says so: *not distributed, notarised or
auto-updated*. There is no `.github/` directory, so nothing runs on a push, and
a release is whatever happens to be in `target/bundle` after the last local
build.

Three things follow from that:

- **Nobody else can install it.** There is no artifact to download, no version
  to point at, and no place where a changelog lives.
- **A person on this Mac has to rebuild to get a fix.** The app can never
  replace itself, so every change costs a rebuild plus a manual copy.
- **"Windows later" has no contract.** ADR-0001 promises a second platform
  crate, but nothing in the repository says what a Windows artifact is called,
  which key it occupies in an update manifest, or which installer format it
  uses — so every one of those decisions will be made twice, in two places,
  when the port finally starts.

The signing story is also deliberately local and deliberately cheap: the bundle
is signed by a self-signed certificate in this Mac's login keychain, and the
Accessibility grant is pinned to that identity (ADR-0004). Distribution has to
keep that identity or every user loses the grant on every update.

## Solution

Four pieces, in the order they depend on each other:

1. **CI on every push and pull request.** The macOS job runs the whole suite
   (`scripts/test.sh`) plus `cargo fmt --check`; it is the first place the
   repository's identity and IPC rules run without a human. The Windows job
   runs what is portable today — the UI checks and `translator-core`'s tests —
   so the core cannot drift away from the second platform while it waits.

2. **Packaging split from signing.** `scripts/package.sh` owns assembly
   (frontend, binary, `.app`, icon, plist assertions) and can build a universal
   binary; `scripts/build.sh` stays the local entry point and delegates to it.
   The release workflow imports the certificate into a temporary keychain and
   calls the same script, so the released bundle is assembled by the same code
   the local one is.

3. **A tag-driven release.** Pushing `vX.Y.Z` on `main` verifies the tag against
   the workspace version, re-runs the suite, builds universal, signs, and
   uploads a **draft** GitHub Release with a DMG (first install), an updater
   tarball, its signature, and `latest.json`. A human installs the draft,
   walks the release smoke table, and publishes. `scripts/set-version.sh` is
   the one writer of the version, and `scripts/version.sh` the one reader.

4. **In-app updates.** The Tauri updater plugin, compiled only into release
   builds through the `updater` feature, checks on launch and every six hours,
   downloads in the background, and waits. A menu-bar row appears when a
   version is ready; Settings has an Updates section with the current version,
   a "Check now" button and the automatic-check switch. Installing restarts the
   app. Only stable releases exist; pre-releases are ignored by construction
   because GitHub's `latest` never points at one.

The Windows contract is written down now, in `.scratch/windows-port/spec.md`,
so its artifact name, manifest key, installer format and CI job have one home
before the port exists. The port itself is not part of this effort.

## User Stories

1. As someone using Neris on a Mac, I want to download one file and end up with
   the app in `/Applications`, so that installing does not mean building.
2. As someone installing for the first time, I want the one Gatekeeper step
   written down exactly, so that "the app is damaged" never happens without an
   explanation next to it.
3. As someone who already granted Accessibility, I want updates to keep that
   grant, so that Neris is not silently broken after every release.
4. As someone using Neris, I want the app to tell me when a new version is
   ready and to restart only when I say so, so that an update never interrupts
   a translation.
5. As someone who does not want the app talking to GitHub, I want to turn the
   automatic check off, so that the only outbound requests are the ones I
   configured.
6. As someone whose update fails to download, I want the reason in Settings,
   so that I can decide whether to try again or download manually.
7. As the person maintaining Neris, I want one command to cut a version and one
   tag to publish it, so that releasing is a ritual rather than a checklist I
   can get wrong.
8. As the person maintaining Neris, I want a release to be a draft until I have
   run the bundle myself, so that CI never publishes something nobody opened.
9. As the person maintaining Neris, I want a local build to be incapable of
   updating itself, so that the bundle I am developing is never replaced by a
   release.
10. As the person who will write the Windows port, I want the artifact name,
    manifest key, installer format and CI job already decided, so that the port
    plugs into the pipeline instead of reopening it.
11. As someone reading the repository, I want the self-signed choice and its
    cost recorded in an ADR, so that "why is this not notarised?" has an answer.
12. As an agent working in the tree, I want the Windows job to run `core`'s
    tests from day one, so that a change that only works on macOS fails in CI.

## Tickets

- [01 — CI workflow](./issues/01-ci-workflow.md) — `fmt`, the macOS suite, and
  the Windows job that runs `core`'s tests.
- [02 — Packaging split, universal builds](./issues/02-packaging-split.md) —
  `package.sh`, `build.sh` delegating, `lipo` for both Apple targets.
- [03 — Version tooling](./issues/03-version-tooling.md) — `version.sh`,
  `set-version.sh`, and the tag check the release workflow runs.
- [04 — In-app updates](./issues/04-in-app-updates.md) — the `updater`
  feature, background check, tray row, Settings section, the settings field.
- [05 — Release artifacts](./issues/05-release-artifacts.md) — DMG, tarball,
  signature, `latest.json`, notes.
- [06 — Release workflow and runbook](./issues/06-release-workflow.md) — the
  tag workflow, the secrets bootstrap, `docs/install.md`, `docs/release.md`.

## Out of scope

- The Windows port itself. Its contract and spike live in
  `.scratch/windows-port/`.
- Notarisation and a Developer ID. Both are a deliberate, reversible *later*
  (ADR-0009); the certificate swap costs every user one Accessibility grant.
- Beta channels. `latest.json` carries one version, stable only.
- Delta updates, rollback to the previous version, and an in-app changelog
  viewer.
