# 11: Bundle identity — one authoring point

**What to build:** The bundle identifier and the version are each authored once. The identifier is authored in the packaged plist, which is the artifact macOS reads; the runtime app-support path is derived from the running bundle instead of being rebuilt from three hard-coded literals, with a compile-time fallback for test binaries that have no bundle; the build script reads the identifier from the plist instead of carrying its own copy; and the Tauri config is asserted against the plist rather than being a second source. The version is authored in the workspace and both plist keys and the config are asserted against it. The assertions run in the test entry point as well as during packaging, so a drift fails the suite before anyone signs a bundle.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] Changing the identifier in one place changes the packaged plist, the runtime app-support path, and the signed identifier together.
- [ ] The three runtime copies of the identifier are gone; the path is derived from the bundle, with a test-binary fallback.
- [ ] The build script holds no identifier of its own and fails packaging when the config disagrees with the plist.
- [ ] The two plist version keys and the config version are asserted against the workspace version, and a mismatch fails before signing.
- [ ] The identity check runs as part of the test entry point, not only inside packaging.
- [ ] Packaging still produces a signed bundle that reports the stable identity, the designated requirement still pins the identifier, and the menu-bar-only key is still present.
- [ ] A one-line note about the rename consequence (a new identifier moves history and settings to a new directory) is recorded in ADR-0004 when this lands.
