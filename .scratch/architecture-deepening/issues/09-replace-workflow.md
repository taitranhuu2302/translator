# 09: Replace — the hide, replace, restore order in one place

**What to build:** Replace becomes a workflow over ticket 08's seam: hide the Quick Translate popup, replace the Selection, and on failure show the popup again so the user can retry or copy. The two failure paths stay distinguishable — a failed Replace, and a popup that cannot be brought back — so the shell can still report the second one without confusing it with the first.

**Blocked by:** 08 (Shell workflow seam — and start at login as its first workflow)

**Status:** ready-for-agent

- [ ] The hide → Replace → restore ordering lives in one workflow, not in a command.
- [ ] The command only hops to the main thread and maps the result.
- [ ] A failed Replace restores the popup; a failure to restore it is reported separately and logged, never swallowed.
- [ ] Tests with a fake presenter assert: success hides once and shows never; a failed Replace hides once and shows once; a presenter that refuses to show is reported as its own outcome.
- [ ] Replacing still writes the translation into the app that owns the Selection, and still refuses when no Selection was captured.
- [ ] The popup's own Replace path and any other path that replaces behave the same way, because they share the workflow.
