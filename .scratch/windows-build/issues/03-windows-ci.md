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

`gh workflow run` needs the workflow on the default branch, so on this branch
the job was triggered by a temporary `push: branches:` entry; it is reverted,
and the workflow listens on `workflow_dispatch` and `v*` again. Whether it is
resolvable by name from `main` is proven by the first manual run after merge.
