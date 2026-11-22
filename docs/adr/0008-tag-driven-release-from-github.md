# Cut releases from a tag, on GitHub, with a draft in between

A release is the only moment the repository's rules meet a person who did not
run the tests, so it is the moment two things must already be true: the tree is
the one that passed, and the bundle is the one the local build would have made.
A `vX.Y.Z` tag on `main` starts `.github/workflows/release.yml`, which refuses
a tag that disagrees with `scripts/version.sh`, re-runs `scripts/test.sh`,
builds universal, signs with the imported certificate, and uploads a **draft**
GitHub Release. Publishing is a human step, taken after the bundle has been
installed and walked against the release checks table.

Packaging stays in `scripts/package.sh` rather than moving to `tauri build`.
The bundle is authored by hand for reasons that have not changed: `LSUIElement`,
an `Info.plist` that is the single authoring point for the identifier
(ADR-0004), an icon built from one source image, and signature assertions that
fail the build rather than a person. `bundle.active` stays `false`; the
workflow calls the same script the local build does, so a released bundle and a
local one cannot be assembled differently.

GitHub Releases is the distribution point because the repository is public:
the assets are free to serve, `latest.json` is an asset, and
`releases/latest/download/latest.json` is a URL that never changes. The updater
needs no token, and a private mirror can be added later without touching the
client.

**Consequences:** a tag is the boundary between "merged" and "released", and
`main` itself is not gated — the release workflow re-runs the suite, so a
broken `main` costs a failed release, not a published one. Pre-releases are
ignored for free: GitHub's `latest` never points at one, which is why there is
one channel rather than two.
