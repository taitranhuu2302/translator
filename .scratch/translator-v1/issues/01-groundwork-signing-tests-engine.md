# 01: Groundwork — stable signing, test target, engine seam

**What to build:** The ground the rest of v1 stands on. The app is signed with a stable identity so the Accessibility permission it is granted survives every rebuild; the project has a test target that runs from the command line; and all non-UI behaviour sits behind a single engine that the views call, with its collaborators injected. Nothing user-visible ships here — this is the prefactor that keeps every later ticket testable.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] The app builds and runs signed with a stable code-signing identity; inspecting the built bundle reports that identity rather than an ad-hoc signature
- [ ] Accessibility permission, granted once, still applies after a clean rebuild and after a rebuild that changes source
- [ ] The test target runs from the command line and passes
- [ ] All non-UI behaviour is reachable through the engine; no view, panel, or menu item calls a provider or the Accessibility layer directly
- [ ] A test drives the engine with fake ports and asserts on an observable outcome, proving the seam works before any feature depends on it
