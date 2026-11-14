# 02: Blank text — one rule at the engine's door

**What to build:** The engine, not its callers, decides what an empty Selection means. Text that is empty after trimming never becomes a provider request: every text entry point on the engine returns one explicit outcome — "there is nothing to translate" — with no provider call and no history row. The same rule covers Translate, Improve, and streaming Improve, which today returns an empty improvement instead. The new outcome is worded once by the notice module from ticket 01.

This closes an opening in the engine's interface, not a visible bug: the UI already trims before calling in three places, so nothing user-visible changes on the existing surfaces. Those checks stay as input affordances (a disabled button is still right) but stop being the enforcement point.

**Blocked by:** 01 (Capture notice — one mapper from a failure to words)

**Status:** ready-for-agent

- [ ] Every engine entry point that accepts text refuses text that is empty after trimming, and none of them consults a provider or records history for it.
- [ ] Streaming Improve returns the same outcome instead of an empty improvement, so all modes share one rule.
- [ ] The outcome is a wire value the UI maps through ticket 01's notice module, with one message.
- [ ] The engine seam proves it with `""`, `"   "`, and `"\n\t "` across every entry point: zero provider calls, zero history rows, one outcome each.
- [ ] A test asserts that a non-blank Selection containing leading or trailing whitespace is still translated (the rule trims to decide, it does not rewrite the text).
- [ ] The existing UI trim checks remain and carry a comment saying the engine enforces the rule and they only improve the affordance.
