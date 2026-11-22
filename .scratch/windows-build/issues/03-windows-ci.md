# Compile gate and installer job

Status: done
Blocked by: 01, 02

## What

1. `.github/workflows/ci.yml`, job `windows`: after `cargo test -p
   translator-core`, add `cargo build -p translator-app`. It runs on every push
   to `main` and every pull request, like the rest of the job.
2. New `.github/workflows/windows.yml`, triggered by `workflow_dispatch` and
   `push: tags: ["v*"]`:
   - checkout, Node 22 with the npm cache, stable Rust, `Swatinem/rust-cache`
   - `npm ci`
   - `npm run build` (the `ui/dist` the embedded frontend needs)
   - `npx tauri build --config crates/app/tauri.conf.json --bundles nsis`
   - `actions/upload-artifact@v4` with `target/release/bundle/nsis/*.exe`,
     `if-no-files-found: error`

## Why

A compile break should fail the pull request that caused it, not wait for
someone to press a button. Packaging a release binary on every push spends
runner minutes for an artifact nobody downloads that often.

## Acceptance

- A PR that breaks `crates/app` on Windows fails the `windows` job.
- A manual run of `windows.yml` finishes green and offers
  `Neris_<version>_x64-setup.exe` as an artifact.
- The job reads its version through `scripts/version.sh`, so the artifact name
  follows `scripts/set-version.sh` with no second copy of the number.

## Result

Run [45029644216](https://github.com/taitranhuu2302/translator/actions/runs/45029644216)
finished green: all fourteen steps, `Build the installer` included (3m51s in
the run before it, which failed on `core-foundation`). The artifact is
`neris-0.2.0-windows-x64`, 4,313,767 bytes, one `Neris_0.2.0_x64-setup.exe`.

`gh workflow run` needs the workflow on the default branch, so before the merge
the job was triggered by a temporary `push: branches:` entry. That entry never
reached `main`. After the merge,
`gh workflow run windows.yml --ref main` resolved the workflow by name and run
[45034250305](https://github.com/taitranhuu2302/translator/actions/runs/45034250305)
(`workflow_dispatch`, `main`, 39b9980) finished green on every step and
uploaded `neris-0.2.0-windows-x64` again, 4,312,356 bytes.

The one thing to watch on a future run is the annotation this one carried:
`actions/checkout@v4`, `setup-node@v4` and `upload-artifact@v4` are being forced
onto Node 24 because Node 20 is deprecated. It works today; it will need a
version bump before GitHub drops that shim.

### The tag half moved into `release.yml`

The first version of this ticket had `windows.yml` listen on `v*` as well and
attach nothing, so `v0.2.0` shipped its `.exe` by hand. That left the file
missing from every later release unless someone remembered, so the split is now:

- `.github/actions/build-windows-installer` — the single recipe for the `.exe`
  (frontend, Rust build, `npx tauri build --bundles nsis`).
- `windows.yml` — `workflow_dispatch` only; artifact, never a release.
- `release.yml` — a `windows-installer` job with `needs: release`; it builds the
  same file and uploads it to the draft the macOS job just created. Waiting on
  the job is what removes the race a second workflow would have had polling for
  a release that does not exist yet.

`scripts/release.sh` writes the "Windows — build only" section into the notes, so
the release page explains the second download without a hand edit.

The release path has not been run yet — the next `vX.Y.Z` tag is its first
exercise, and the thing to look for is the `.exe` sitting on the draft next to
the DMG before anyone publishes. The dispatch path was re-run after the
extraction: run
[45036725363](https://github.com/taitranhuu2302/translator/actions/runs/45036725363)
shows the job step as `./.github/actions/build-windows-installer`, finished
green, and uploaded `neris-0.2.0-windows-x64` (4,315,883 bytes).
