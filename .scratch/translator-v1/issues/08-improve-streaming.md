# 08: Improve

**What to build:** The Improve tab takes an English sentence the user wrote and returns two things: the same sentence corrected, and a better version of it. The text streams in as the model produces it, and the two parts are separated so either can be copied on its own.

**Blocked by:** 06, 07

**Status:** ready-for-agent

- [ ] Improve returns the corrected sentence and the better sentence as two clearly separated parts, never as unflagged prose
- [ ] The corrected part preserves the author's meaning and register; the better part is more natural or more concise
- [ ] Text streams into the pane while the model is answering
- [ ] Malformed model output produces an error instead of mixing the two parts together
- [ ] Without the key Improve needs, the app says which key is missing instead of failing silently
- [ ] A failed Improve leaves the user's input intact
