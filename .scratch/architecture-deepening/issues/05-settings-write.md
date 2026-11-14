# 05: Settings write — build next, save, mirror

**What to build:** One module owns the settings write protocol. Its interface is a single `change(mutation)`: it reads the current settings, builds the next whole object, saves it, and returns what the view may mirror into its shown state. The rule for mirroring is stated once, replacing the three different rules in use today (mirror only when there is no problem, mirror only when the value is live and saved, mirror unconditionally). Failure comes back with one sentence per failure class instead of a sentence per call site.

Target, shortcut rebind, reset, and Azure region all go through it in this ticket; the provider key state follows in ticket 06, which is blocked by this one.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] A settings change cannot be written without going through `change(mutation)`; no view builds a whole settings object inline to save it.
- [ ] The module saves a complete object every time, never a partial one.
- [ ] The mirror rule lives in the module: a view shows the new value only when the module says it may, with reset expressed as a mutation that is unambiguously saved.
- [ ] Failure returns one sentence per failure class, and the view shows it without adding wording.
- [ ] Tests cover each of the four mutations: a successful save mirrors, a failed save does not mirror, reset mirrors, and a mutation that changes nothing still leaves shown and saved equal.
- [ ] No settings view keeps its own copy of the mirror condition or of a failure sentence.
