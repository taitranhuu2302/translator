# 02: Packaging split from signing, and universal builds

**What to build:** `scripts/package.sh` owns everything `scripts/build.sh` did
except the assumption that this Mac holds the certificate: dependency install,
frontend build, placeholder refusal, binary build, `.app` assembly, the icon,
the plist assertions and the signature assertions. It takes `--universal`,
which builds `aarch64-apple-darwin` and `x86_64-apple-darwin` and `lipo`s the
two binaries, so a release runs on both Apple architectures. `build.sh` becomes
a one-line delegation, so the documented local command and the released bundle
are assembled by the same script. Signing still uses
`$TRANSLATOR_SIGNING_IDENTITY` (defaulting to the local identity), so CI only
has to import a certificate under that name.

**Blocked by:** None (can start immediately)

**Status:** in-progress

- [ ] `./scripts/build.sh` produces the same signed `target/bundle/Neris.app`
      as before, with every existing assertion intact.
- [ ] `./scripts/package.sh --universal` produces a bundle whose `Mach-O` is
      fat (`lipo -info` names both architectures).
- [ ] The `cc`-compiled Objective-C objects build for each target, not just the
      host's.
- [ ] A missing signing identity still fails before the build, naming ADR-0004.
- [ ] The placeholder check and the plist/`CFBundleExecutable` checks still
      refuse a wrong tree.

## Implementation

`package.sh` now owns the whole assembly: dependency install, frontend build,
placeholder refusal, binary build, `.app` assembly, icon, plist assertions and
signature assertions. `build.sh` is `exec ./scripts/package.sh "$@"`.
`--universal` adds both Apple targets to the `cargo build` and `lipo`s the pair;
the per-target `cc` objects go to separate `OUT_DIR`s because they are built by
each target's own invocation. Signing still reads `$TRANSLATOR_SIGNING_IDENTITY`
and defaults to the local identity.

Verified on 2026-11-22: `./scripts/package.sh --updater` refuses the placeholder
public key before building. The universal build, the `lipo -info` result and the
per-target `cc` objects are unverified — they need a full build run.
