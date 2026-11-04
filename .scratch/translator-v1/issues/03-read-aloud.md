# 03: Read Aloud

**What to build:** Selecting English text in any app and pressing the Read Aloud shortcut speaks it in the system voice. The Slice is complete on its own: it needs no translation provider, no key, and no network, which is why it comes before anything that talks to a server.

**Blocked by:** 02

**Status:** ready-for-agent

- [ ] Selecting English text and pressing `⌃⌥S` speaks the Selection with the system speech voice
- [ ] Read Aloud speaks only the Selection and never a translation
- [ ] With nothing selected, the shortcut produces no speech and no error dialog
- [ ] Speech does not block the app: another shortcut still works while it is speaking, and the shortcut works again after it ends
- [ ] With a Vietnamese Selection, it speaks with a Vietnamese voice when one is available and fails visibly when none is
