# Compile gate and installer job

Status: ready-for-agent
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
