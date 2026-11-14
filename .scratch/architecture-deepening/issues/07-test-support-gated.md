# 07: Test support — keep the fakes, stop shipping them

**What to build:** The fakes stay in the same module with the same behaviour, but they stop being part of what the core ships. They are exposed only under test configuration, so the library's interface is the seven ports and the engine, and a consumer cannot accidentally build on a second implementation of everything. As part of the same move, the fake history store is corrected to match the real one: it trims the query and treats `%`, `_`, and `\` literally, so a test that passes is evidence about the app.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] The fakes compile only for tests (a test-support feature enabled by dev-dependencies); nothing in the shipped interface exposes them.
- [ ] Every current user of the fakes still builds: the engine seam test, the app's tests, and the core's unit tests.
- [ ] The in-memory history store and the macOS history store agree on trimming and on treating `%`, `_`, and `\` literally.
- [ ] A parity test drives both stores through the history port with the same queries — empty, whitespace, `100%`, `a_b`, a backslash — and asserts the same results.
- [ ] The fakes keep one home; no fake is duplicated into a consuming crate.
- [ ] The test entry point still runs everything with one command, and a build with the feature off does not reference test support.
