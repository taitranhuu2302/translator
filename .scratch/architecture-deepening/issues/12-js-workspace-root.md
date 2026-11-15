# 12: JavaScript interface — one root, no dead tooling

**What to build:** The repository has one JavaScript interface. The root becomes an npm workspace containing the UI package, so `npm install`, `npm test`, and `npm run build` at the root cover the UI and there is one obvious command per intent. The unused shadcn devDependency is removed from the root — the MCP entry point keeps invoking it through npx. The frontend placeholder marker becomes one described contract between the app build and the build script rather than a magic string duplicated on both sides.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] `npm install`, `npm test`, and `npm run build` at the root do the right thing for the UI, and the existing per-package commands still work.
- [x] The root manifest lists only tooling the repository actually runs; shadcn is gone and nothing else depended on it.
- [x] The root install no longer pulls the transitive tree that shadcn brought in.
- [x] The placeholder marker is defined once, and the build script's refusal names the same value the app build writes.
- [x] The test entry point and the build script still pass end to end, including the refusal to package a bundle holding the placeholder.
- [x] The documented install and build commands in the README match what the repository now runs.

## Implementation

The root `package.json` is now the repository's only npm entry point: a private
workspace root declaring `workspaces: ["ui"]` and forwarding `build`,
`typecheck` and `test` to the `ui` workspace. `npm install` at the root installs
the root dev tooling and the whole UI into one `node_modules`; one root
`package-lock.json` replaces the root lock plus `ui/package-lock.json`. The
per-package commands still work: `npm --prefix ui run <script>` resolves
hoisted binaries through the parent `node_modules/.bin`.

`shadcn` is gone from the root devDependencies. Nothing in the repository
imported it, and `.mcp.json` needed no change: it starts the server as
`npx shadcn@latest mcp`, which does not rely on a local install. The root
lockfile no longer contains `shadcn`, `@babel/*`, `ts-morph` or any of the rest
of its tree.

The placeholder marker is authored once, in `ui/placeholder-marker.txt`
(`<!-- translator-ui-placeholder -->`). `crates/app/build.rs` takes that value
via `include_str!` when it writes `ui/dist`'s placeholder, and
`scripts/build.sh` reads the same file's contents for its refusal. The build
script fails if the file is empty, so a broken definition cannot silently pass
the check. No other source or script file carries the value.

`scripts/test.sh` and `scripts/build.sh` now install from the root with
`npm ci`; their run commands stay per-package, so the suite itself keeps
exercising the compatibility path. Ticket 11's identity assertions are
untouched and still run before the install.

The README's Build and Develop sections document root `npm install`, `npm test`
and `npm run build`, and note that the per-package commands remain available for
a UI-only loop.

## Verification

| Command | Result |
| --- | --- |
| `npm install --no-audit --no-fund` (root, no `node_modules`) | exit 0, 58 packages added; `ui` linked as a workspace |
| `rm -rf node_modules ui/node_modules && npm ci --no-audit --no-fund` | exit 0, 59 packages from the single root lockfile |
| `npm ls shadcn` | `translator@ ... -- (empty)` |
| Lockfile comparison | old: root 237 packages + UI 83 (311 unique); new: one lock with 95 packages; 0 of the old 32 `shadcn`/Babel/ts-morph entries remain |
| `npm test` (root) | 11 files, 161 tests passed |
| `npm run typecheck` (root) | exit 0 |
| `npm run build` (root) | exit 0, writes `ui/dist` |
| `npm --prefix ui run test` | 11 files, 161 tests passed |
| `./scripts/test.sh` | exit 0: `identity ok: com.taitran.translator, version 0.1.0`, typecheck, 161 UI tests, UI build, IPC boundary, cargo tests (34 + 121 + 3 + 78 + 24 + 2 passed) |
| `./scripts/build.sh` | exit 0, packages and signs `target/bundle/Translator.app` |
| Deliberate placeholder: `rm -rf ui/dist && cargo build --release -p translator-app --features custom-protocol` (writes and embeds the placeholder), then temporarily skip the frontend-build lines in `build.sh`, then `./scripts/build.sh` | exit 1: `error: ui/dist still holds the compile-time placeholder ("<!-- translator-ui-placeholder -->"); refusing to package.` No bundle produced. |
| Revert: `git checkout -- scripts/build.sh`, then `./scripts/build.sh` | exit 0; `strings` on the packaged binary shows no placeholder and does show the built asset `assets/main-BImUG2tw.js` |
| `git merge spec/architecture-deepening --no-edit` | conflicts only in `scripts/build.sh` and `scripts/test.sh`; both resolved keeping ticket 11's identity step before the workspace install; suite re-run green |

No behaviour was checked by hand and the app was not launched. The packaged
binary was inspected for the placeholder marker and for the built asset path,
which is the part no test can observe without launching it.

## Comments

- 2026-11-15 — npm hoists almost everything to the root, so `ui/node_modules`
  now holds only the nested `@vitejs/plugin-react` npm chose; `npm ci --prefix
  ui` is no longer an entry point because `ui/package-lock.json` is gone. Use
  the root install.
- 2026-11-15 — `.mcp.json` already used `npx shadcn@latest mcp`, so removing the
  devDependency needed no MCP change; the on-demand download is the intended
  behaviour.
