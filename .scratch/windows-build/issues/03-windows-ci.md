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
