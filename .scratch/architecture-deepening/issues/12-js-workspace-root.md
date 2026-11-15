# 12: JavaScript interface — one root, no dead tooling

**What to build:** The repository has one JavaScript interface. The root becomes an npm workspace containing the UI package, so `npm install`, `npm test`, and `npm run build` at the root cover the UI and there is one obvious command per intent. The unused shadcn devDependency is removed from the root — the MCP entry point keeps invoking it through npx. The frontend placeholder marker becomes one described contract between the app build and the build script rather than a magic string duplicated on both sides.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] `npm install`, `npm test`, and `npm run build` at the root do the right thing for the UI, and the existing per-package commands still work.
- [ ] The root manifest lists only tooling the repository actually runs; shadcn is gone and nothing else depended on it.
- [ ] The root install no longer pulls the transitive tree that shadcn brought in.
- [ ] The placeholder marker is defined once, and the build script's refusal names the same value the app build writes.
- [ ] The test entry point and the build script still pass end to end, including the refusal to package a bundle holding the placeholder.
- [ ] The documented install and build commands in the README match what the repository now runs.
