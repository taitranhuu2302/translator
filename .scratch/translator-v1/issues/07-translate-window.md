# 07: Translate Window

**What to build:** The Translate Window becomes real. Type or paste text, press Enter, and the translation fills the other pane. The result can be copied or selected in part, and one translation can be reversed without changing the saved Target. This is the surface for text too long for a popup.

**Blocked by:** 04

**Status:** ready-for-agent

- [ ] Typing or pasting text and pressing Enter translates it into the result pane
- [ ] The result is selectable, and copying the result copies only the translation
- [ ] A swap control reverses direction for that one translation and leaves the saved Target as it was
- [ ] Translating from the window records the same history and cache behaviour as Quick Translate
- [ ] The window remembers its frame between launches
- [ ] A failure shows in the result pane with the provider named, and does not discard the input
