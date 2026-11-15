# Architecture deepening

Status: ready-for-agent

## Problem Statement

The app works and the crate split holds: product logic is in `core`, the AppKit layer is in `platform-macos`, the shell is in `app`, and the UI talks through one IPC module. What is not organised is where **decisions** live. The same rule is written several times, in several places, in slightly different words, so learning one rule means reading four or five modules — and the copies have already started to disagree:

- A whitespace-only Selection is refused by three separate `trim()` checks in the UI and by an `improve` guard in the engine, but the engine's translation entry points have no such rule at all, so the interface permits a request the UI has to remember to prevent.
- A provider failure is turned into a `ProviderError` by three near-identical mappers, into a message by two more places, and into a notice kind by a third — and the two message tables have already drifted apart in wording for the same failure.
- "The newest answer wins" is enforced by three unrelated counters in three modules, and each reducer's own guard is only safe because its caller remembers to pre-check.
- Settings are saved from five places, each with its own rule about when the saved value may be mirrored into the UI's state, and its own failure sentence.
- The bundle identifier is written six times and the version four times, so the build script carries hand-written guards whose only job is to catch the copies drifting.
- The shell's real ordering — hide the popup, Replace, put it back on failure; read the login flag, change it, save it, undo both when the save fails — lives inside Tauri commands that cannot run without an `AppHandle`, so the sequences that can actually break have no tests, while the pure plans that cannot break are fully tested.

For a person reading the tree this is the "confusing structure" they notice: not too many files, but too many **shallow** modules — each with an interface about as wide as what it does, and none of them with enough **depth** to be the one place you look. For an agent working in the tree it is worse, because every one of those copies is a place to make an edit that is correct locally and wrong globally.

## Solution

Ten deepenings, grouped into four workstreams, each of which gives one rule exactly one **home**:

1. **`core` owns failure and emptiness.** One provider failure table maps transport and status outcomes for all rungs, with each provider's differences expressed as data rather than as a fourth copied match. One blank rule at the engine's door decides what an empty Selection means, for every mode.
2. **`app` owns its workflows.** The ordering behind start-at-login, Replace and shortcut rebinding moves out of the Tauri commands into shell workflows that run over **ports**, leaving each command to do only the hop to the main thread.
3. **The UI owns one owner per rule.** Answer freshness, capture wording, the settings write protocol and the provider key row each collapse from several copies into one module. Components draw what those modules return and add nothing of their own.
4. **The repository owns its identity.** One authoring point for the bundle identifier and the version, with every other place deriving from it or asserting against it; one JavaScript interface at the root instead of two.

Nothing user-visible is added. A few behaviours become more consistent: every surface says the same thing about the same failure, a late answer can never land in a pane opened after it, and saving a setting follows one rule everywhere. The visible payoff is that a change to any of those rules is one edit in one module, and the tests for it run without a running app, a network, or a keychain.

## User Stories

1. As someone using Quick Translate, I want a Selection that is only whitespace to be treated as nothing selected, so that I see one clear message instead of waiting for a translation that cannot exist.
2. As someone using the Translate Window, I want blank input to be handled by the same rule as Quick Translate, so that the two surfaces never disagree about what counts as nothing.
3. As someone whose Replace fails, I want the popup to come back so I can retry, and I want that ordering to be the same every time whichever surface starts the Replace.
4. As someone turning start-at-login on, I want a failure to save the preference to leave the login item exactly as it was, without leaving the two disagreeing.
5. As someone rebinding a shortcut, I want a failed save to restore both the old binding and the old route, so that I never end up with a shortcut that does nothing.
6. As a developer reading the provider modules, I want transport and status failures mapped in one table, so that reading one module tells me how every rung fails.
7. As a developer adding a provider, I want to supply a code table and a wire shape rather than a fourth copied mapper, so that the new rung cannot quietly differ from the others.
8. As a developer changing what a rate limit means, I want to change it once, so that all rungs and all messages move together.
9. As a developer changing the blank rule, I want to change it once, at the place where text enters the engine, so that no caller can bypass it.
10. As an agent writing a new caller of the engine, I want the engine's interface to refuse blank input itself, so that I cannot introduce a network request for a space by forgetting a trim.
11. As a developer opening the provider modules, I want each one to contain only its wire shape and body decoding, so that I can find what is provider-specific by looking at one file.
12. As a developer reading the shell, I want each Tauri command to be a thin hop, so that I can tell shell plumbing from product decisions at a glance.
13. As a developer changing the start-at-login sequence, I want it in one function over ports, so that I can read the ordering and its rollback in one place.
14. As a developer changing what Replace does on failure, I want it in one function with a presenter port, so that I do not have to reason about Tauri's event loop to see it.
15. As a developer writing a test for a shell workflow, I want to drive it with fake ports, so that the test needs no running app and no Accessibility grant.
16. As a developer reading the shortcut code, I want the apply sequence — register, unregister, set route, save, undo — in one module with a named seam, so that I can see the ordering rather than infer it.
17. As a developer changing the rollback after a failed rebind, I want a test to fail when I get the order wrong, so that a regression is caught before I ship it.
18. As a developer resetting shortcuts, I want the reset to reuse the same apply sequence, so that the two paths cannot diverge.
19. As a developer reading `core`, I want the shipped interface to be the seven ports and the engine, so that I am not handed a second implementation of everything to read around.
20. As a developer relying on the fakes, I want them compiled only for tests, so that a fake cannot become a de-facto part of the public interface.
21. As a developer testing history search, I want the fake and the real store to agree on trimming and wildcard handling, so that a test that passes is evidence about the app.
22. As a developer adding a new port, I want to add one fake, in one place, gated the same way, so that test support stays a single module.
23. As someone using Quick Translate, I want an answer from a previous open never to appear in the popup after I reopen it, so that I never Replace the wrong text.
24. As someone using the Translate Window, I want a reopened history entry never to be overwritten by an answer for what I typed before, so that the pane I am reading stays mine.
25. As someone using Improve, I want a late chunk from an abandoned attempt to be dropped, so that a streamed answer cannot mix two attempts together.
26. As a developer adding a new asynchronous answer path, I want to call one token module instead of inventing a counter, so that freshness is correct by construction.
27. As a developer reading the reducer tests, I want the late-answer case expressed as a token that is no longer current, so that I can test it without simulating component lifecycles.
28. As a developer changing the wire request id, I want the conversion to live in one adapter, so that I do not have to find every sender.
29. As someone whose Selection cannot be read, I want the same explanation whether I am in Quick Translate or the Translate Window, so that I do not wonder whether one of them is broken.
30. As someone with secure input active, I want one clear message, not two slightly different ones depending on which window I am looking at.
31. As a developer adding a new capture failure, I want to add it to one mapper, so that both surfaces handle it and neither can be forgotten.
32. As a developer changing a failure's wording, I want to change it in one module, so that the app never speaks in two voices about one cause.
33. As someone changing the Target, I want the change to appear in the UI only when it has actually been saved, so that I never see a setting that did not stick.
34. As someone resetting settings, I want the UI to reflect the saved state immediately and honestly, so that I can trust what I see.
35. As a developer adding a setting, I want to describe the mutation and let one module build, save and mirror, so that I cannot forget the mirror rule or write a partial object.
36. As a developer reviewing a settings change, I want the whole-object save and its failure sentence in one place, so that I can see what the user will be told when it fails.
37. As someone configuring an Azure key, I want the same save, clear and status behaviour as for a Groq key, so that the two rows cannot behave differently.
38. As a developer adding a third provider key, I want to compose a row rather than copy four handlers and a block of markup, so that the cost of a provider is its wire shape and nothing else.
39. As a developer changing key handling, I want one module to change, so that the Azure and Groq paths stay in step.
40. As a developer changing the bundle identifier, I want one authoring point, so that the plist, the Tauri config and the runtime paths cannot drift.
41. As a developer changing the version, I want one authoring point, so that the packaged plist, the config and the crate cannot disagree.
42. As a developer running the test suite, I want an identity check that runs before packaging, so that a drift is a failing test rather than a lost Accessibility grant.
43. As an agent reading the repository root, I want one JavaScript interface, so that "install, test, build" is one obvious command rather than a guess between two manifests.
44. As a developer installing dependencies, I want no unused developer tooling, so that the install stays small and the manifest says what the project actually uses.
45. As a developer reading the packaged-build path, I want the placeholder guard to describe one relationship rather than two modules sharing a magic string.
46. As an agent asked to change how translation failures are reported, I want to find the failure table, the notice mapper and the two surfaces it feeds without reading the whole tree, so that the change stays local.
47. As a reviewer, I want each deepening to be its own change with its own tests, so that one rollback cannot take another with it.

## Implementation Decisions

### Workstream A — `core` owns failure and emptiness

**A1. One provider failure table.** A new module in `core`'s providers side owns every conversion from a transport or HTTP outcome into a `ProviderError`. Its interface is small and takes the provider as an argument:

- transport failure → `Offline` or `Network { provider, message }`
- rate limit → `Unavailable { reason }` with the provider's sentence
- server error status → the provider's chosen family
- other statuses → `MalformedResponse { provider, message }`

Each rung supplies its policy as **data**, not as a match arm: which statuses mean authentication, which error codes map to which `AuthFailure` or `QuotaExceeded`, what its server-error family is, and its rate-limit sentence. The table is the only place the shared statuses are enumerated.

The existing per-provider decisions are preserved exactly and become data: Azure's 403/403001 quota split and 403/403000 resource-not-authorized cause, Azure's and Groq's 5xx → `Network` versus MyMemory's 5xx → `Unavailable`, and MyMemory's body-level quota detection. A rung's module keeps only what is genuinely its own: request shape, response decoding, and its code table.

**A2. One blank rule at the engine's door.** The engine gains a single rule, applied before any provider is consulted: text that is empty after trimming is not a translation request. It produces one new, explicit outcome — an engine error meaning "there is nothing to translate" — rather than a provider call, a history row, or an empty success.

- The rule applies to every text entry point on the engine, including the streaming Improve path, which today returns an empty `Improvement` instead. One rule, one outcome, every mode.
- The UI already refuses blank input before calling the engine, in three separate places. Those checks stay as cheap input affordances (a disabled button is still correct UX) but they stop being the enforcement point; the engine's interface no longer permits the request at all.
- The new outcome is a wire value the UI must map. It is handled by the notice module in C2, which is the first consumer of it.

**A3. One owner for provider messages.** The `Display` implementations and per-provider reason sentences move into the failure table's neighbourhood so that a provider's words are written once, next to the code table that selects them. The rung modules do not format user-facing sentences.

### Workstream B — `app` owns its workflows

**B1. One shell workflow seam.** The `app` crate gains a module of workflows, each a plain function over a small set of `app`-local ports plus the core `SettingsStore`:

- `LoginItem` — read and set the start-at-login state
- `PanelPresenter` — hide and show the Quick Translate popup
- `KeyRegistry` — register and unregister a global shortcut
- `ShortcutRoute` — set the path a shortcut calls

The ports are declared in `app`, not `core`: they describe the shell, not the product (ADR-0001 keeps product logic in `core`). One seam, not one per command.

**B2. Start at login becomes a workflow.** The read → change → save → roll-back sequence moves behind that seam as one function. The Tauri command becomes: hop to the main thread, call the workflow, map its result. The rollback is stated once, in order, and is testable with a fake `LoginItem` that refuses to change and a fake `SettingsStore` that refuses to save.

**B3. Replace becomes a workflow.** Hide the popup → Replace the Selection → on failure show the popup again. The workflow takes the engine's Replace operation and a `PanelPresenter`; the command only performs the main-thread hop. The two failure paths (Replace failed, panel could not be re-shown) stay distinguishable in the workflow's result so the command still logs the second one.

**B4. Shortcut rebinding becomes a workflow.** The apply sequence moves to the seam:

1. register the new binding
2. unregister the old one
3. set the route
4. save the settings
5. on any failure after step 2: unregister the new binding, re-register the old one, restore the old route

Shortcut reset reuses the same sequence rather than repeating it. The pure planning function and its types are unchanged; only the apply half moves.

**B5. Consistent main-thread timeouts.** The three call sites currently pass 1 s, 2 s and 2 s for the same kind of hop, and one path bypasses the helper entirely. The workflows take the timeout as one named constant so the shell has one answer to "how long may the main thread take". This is a small decision inside B, not a separate workstream.

### Workstream C — the UI owns one owner per rule

**C1. Answer token.** A new pure module owns "the newest answer wins". Its interface is two functions: one issues a token for a new attempt, one answers whether a token is still current. A token carries both a session identity and a sequence number, so it survives the case that breaks a bare counter: the popup is reopened, a new session starts, and the previous session's answer arrives afterwards.

- `Quick Translate`'s reducer stores the token instead of relying on `phase === "translating"`, which cannot distinguish two sessions that are both translating.
- The `Translate Window`'s Translate and Improve reducers store the token instead of the caller's `requestId`; `restored` remains valid from any phase because it is not an answer to a request.
- `ipc` converts the token to and from the wire's string request id in one place, so no sender parses or invents ids.
- The component-level `session` ref in the popup disappears; the reducer's own rule is sufficient.

**C2. Capture notice.** One module turns an `EngineError` into a notice model: a kind, a title, a detail, and at most one action. `Quick Translate` and the `Translate Window` both draw that model and add no wording of their own.

- The existing drift is resolved by choosing one text per case and deleting the other. The Accessibility detail currently differs between the two surfaces for the same cause; one survives.
- The model is also where the new blank-text outcome from A2 is worded.
- The kind used for styling is derived from the same model, so the wire value is mapped once rather than three times.

**C3. Settings write.** One module owns the write protocol. Its interface is a single `change(mutation)` that:

1. reads the current `Settings`
2. builds the next whole object
3. saves it
4. returns what the view may mirror into its shown state

The rule for mirroring is stated once. Today three different rules exist — mirror only when there is no problem, mirror only when the value is live and saved, mirror unconditionally — and they are reconciled into one, with the reset case expressed as a mutation that is unambiguously saved.

All five write sites go through it: Target, shortcut rebind, reset, Azure region, provider key state. Each site's own failure sentence is replaced by the module's one sentence per failure class.

**C4. Provider key row.** One component-module renders a provider's key row and owns save, clear, busy, and notice. Its interface is the provider plus the row's state; the Azure adapter additionally carries the region. Azure and Groq become two configurations of it, not four copied handlers and two blocks of markup.

### Workstream D — the repository owns its identity

**D1. One authoring point for identity.** The bundle identifier is authored in the packaged plist, which is the artifact macOS reads. Everything else derives from it or asserts against it:

- the runtime app-support path is derived from the running bundle's identifier instead of being rebuilt from a hard-coded literal in three modules, with a compile-time fallback for test binaries that have no bundle
- the build script reads the identifier from the plist instead of carrying its own copy
- the Tauri config keeps its identifier as a build-time input, and the build script asserts it equals the plist's — the drift guard becomes an assertion against the single source rather than a comparison of two copies

**D2. One authoring point for the version.** The workspace version is the source; the packaged plist's two version keys and the Tauri config's version are asserted against it by the build script. A mismatch fails packaging, before signing.

**D3. One inline identity check for the test suite.** The identity assertions run as part of the test entry point, not only inside packaging, so a drift is caught by the suite rather than by the first manual packaging run.

**D4. One JavaScript interface.** The root becomes an npm workspace containing the UI package:

- `npm install`, `npm test`, `npm run build` at the root cover the UI, so there is one obvious command per intent
- the unused `shadcn` devDependency is removed; the MCP entry point keeps invoking it through `npx`
- the frontend placeholder marker is described as one contract between the build script and the app build: the marker string is defined once and the check names the same constant's value

**D5. Test support is compiled for tests only.** `core`'s fakes stay in the same module, with the same behaviour, but are exposed only under test configuration (a `test-support` feature enabled by dev-dependencies). The public interface becomes the seven ports and the engine.

A parity test covers the divergence that already exists: the in-memory history store and the macOS history store must agree on trimming and on treating `%`, `_` and `\` literally. The test lives where both implementations are reachable and drives both through the same `HistoryStore` interface.

### Sequencing

The workstreams touch disjoint modules except for two points: A2 introduces the wire value that C2 must word, and B1's seam is the only new seam in the whole effort. Suggested order, each landing as its own change:

1. A1–A3 (core failure and blank rule) — no UI dependency beyond the new outcome.
2. C2 (capture notice) — consumes A2's outcome; unblocks nothing else.
3. C1 (answer token), C3 (settings write), C4 (provider key row) — independent of each other.
4. B1–B5 (shell workflows) — the largest, and the only one that adds a seam.
5. D5 (test-support gate and parity test) — last among the code changes, because it compiles the fakes the earlier tests rely on.
6. D1–D4 (repository identity and workspace root) — mechanical, independent of the code workstreams.

## Testing Decisions

**What makes a good test here.** A test drives a module through its interface and asserts on what the caller can observe: the value returned, the sequence of calls the module made through its ports, or the state a reducer produced. It does not assert on private helpers, on the number of times an internal function ran, or on the shape of an intermediate value that is not part of the interface. The test that would survive a rewrite of the module's internals is the test worth having. Where a rule was previously tested through a pure helper extracted for testability, the test moves to the sequence that uses the helper.

**Seam 1 — the engine seam (existing, `core`).** The engine is driven with fake ports, as `engine_seam.rs` already does. It covers:

- the blank rule for every text entry point: no provider call, no history row, one explicit outcome
- the failure table end-to-end through the ladder: a fake provider answering 401 skips its rung for the session; 429 does not; a 5xx maps to the family its policy declares
- the provider failure table's own units, tested through the ladder rather than through the mappers

**Seam 2 — the shell workflow seam (new, `app`).** One new seam, the `app`-local ports described in B1. It covers:

- start-at-login: success, login-item refusal, save failure, and the rollback asserting both the login item and the saved settings are where they started
- Replace: success, Replace failure restoring the popup, and a popup that fails to come back being reported separately
- shortcut rebinding: the full apply order, and the rollback after a forced save failure
- shortcut reset reusing the same sequence

Prior art for the fakes: the inline test modules in the shell already construct plain test doubles for pure planning; the new ones follow the same style. No Tauri runtime, no AppKit, no keychain.

**Seam 3 — the UI's pure modules (existing pattern, Vitest).** Every UI change is tested the way `improveWindow`, `popupState`, `settingsView`, `providerStatus` and `translateWindow` already are: a pure module plus injected effects, driven by explicit events.

- answer token: a late answer from a previous session does not land, including the reopen-with-answer-in-flight case that the current popup test does not cover
- capture notice: every `EngineError` variant and every capture failure produces one model, and the model is the only source of wording
- settings write: each mutation saves a whole object, mirrors only when the module says so, and reports one failure sentence
- provider key row: save, clear, empty draft, busy, and failure for both providers through the same module

**Seam 4 — the packaged bundle (existing, the build script).** The identity assertions run as one inline check in the test entry point. It covers: plist identifier equals the config identifier, both plist versions equal the workspace version, and the runtime path derivation reads the same identifier. Prior art: the existing drift guards the build script already runs.

**Not tested.** Component layout, the panel's AppKit behaviour, and the wire encodings of third-party APIs beyond the decoding the rungs already own.

## Out of Scope

- Re-litigating the crate split (ADR-0001), the provider ladder and its session-skip rule (ADR-0003), packaging as part of the build (ADR-0004), or the non-activating panel (ADR-0005). This spec gives existing decisions one home; it does not change them.
- Behaviour changes beyond those named: no new shortcuts, no new providers, no change to which provider answers first, no change to the Improve output shape.
- The 12 local branches with unique commits and the `target/` directory's disk use. Both are housekeeping decisions the user has not made.
- Migrating off npm, or restructuring the Rust workspace into more crates.
- Rewriting the IPC module's surface. It keeps one method per command; only the request-id conversion and the new error value are added.
- The intermittent history-test failure caused by two parallel tests landing on the same temp path. It is a real defect and should be fixed first, separately, but it is not part of this effort.

## Further Notes

**A correction to the review's framing.** The review said a whitespace-only Selection reaches Azure and MyMemory over the network. That is not reachable through the current UI: the popup, the Translate Window's Translate pane and its Improve pane each trim before calling the engine. What is true is that the engine's interface permits it and nothing in the engine refuses it, so the rule exists only in the callers. The defect is a missing rule at the seam, not a live network call — the deepening is the same, and the ticket should be written as such so nobody goes looking for a network trace that is not there.

**Why this is one spec and not ten.** The ten deepenings share one seam decision (one new seam, in `app`) and one testing vocabulary, and the sequencing above has real edges between them. Splitting them into separate specs would duplicate the seam and testing sections ten times. They are still ten separate changes, and the tickets should be cut that way.

**The deletion test.** Each item is worth doing only if deleting the copy concentrates the rule rather than moving it. A1 passes: the three mappers disappear and one table appears. C4 passes: four handlers become one module plus two configurations. D4 passes: one manifest disappears. D1 passes only if the runtime derivation removes the three literals; if it merely re-points them at a constant, stop and reconsider.

**ADR note for D1.** Deriving the app-support path from the running bundle changes where the app writes on disk if the bundle identifier ever changes. That is the intended behaviour — the identifier is the identity — but it means a rename moves the history and settings files. Record that as a one-line note in ADR-0004 when the change lands, so a future rename is not a surprise.

**Vocabulary.** This spec uses the deep-module vocabulary deliberately: every item above turns a **shallow** module into a **deep** one — more behaviour behind a smaller **interface**, at a **seam** where a test can reach it. The domain words — Selection, Language Pair, Target, Replace, Improve, Quick Translate, Translate Window, Read Aloud — keep their meanings from `GLOSSARY.md`. No new domain terms are introduced; the module names above are architecture, not domain language, and do not belong in the glossary.
