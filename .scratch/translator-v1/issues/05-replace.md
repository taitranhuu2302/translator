# 05: Replace

**What to build:** The user can overwrite the Selection with its translation in place. Pressing Enter in the Quick Translate popup does it after a look; `⌃⌥R` does it directly with no popup. It works both where the app accepts inserted text and where it only accepts a paste, and it tells the truth when it cannot write at all.

**Blocked by:** 02, 04

**Status:** ready-for-agent

- [ ] Enter in the Quick Translate popup Replaces the Selection with the translation and closes the popup
- [ ] `⌃⌥R` translates and Replaces in one step, with no popup
- [ ] Where the focused element accepts text insertion, Replace goes through it and the surrounding text is unchanged
- [ ] Where it does not, Replace uses the clipboard and restores the previous clipboard contents afterwards
- [ ] When Replace cannot happen at all, the failure is visible and the document is left unchanged
- [ ] After Replace, the Selection in the source app covers the new text rather than extending past it
- [ ] Replace is refused, and says so, when secure event input is active
