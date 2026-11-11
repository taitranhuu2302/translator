# 04: Quick Translate with the keyless provider

**What to build:** Selecting Vietnamese text and pressing `⌃⌥D` opens a popup beside the cursor showing the English translation, with the source app still focused and the caret still where the user left it. It works on first launch, before any key exists, because it uses the keyless provider. This is the slice that proves the whole Quick Translate shape: popup placement, focus behaviour, direction, chunking, and caching.

**Blocked by:** 02

**Status:** ready-for-agent

- [ ] `⌃⌥D` with a Selection opens the popup next to the cursor and shows a translating state immediately, before any network response
- [ ] The translation replaces the translating state in the same popup
- [ ] The popup becomes key so Enter and Escape reach it, without activating the app: the source app keeps its menu bar and stays frontmost
- [ ] When the popup closes, typing goes back to the source app with the caret where it was
- [ ] With Target set to English, a Vietnamese Selection translates to English and an English Selection translates to Vietnamese
- [ ] Text longer than the keyless provider's per-request byte cap is split and rejoined in order, with Vietnamese diacritics intact
- [ ] Translating the same Selection twice returns the second result without a network request
- [ ] Escape dismisses the popup and leaves the document untouched
- [ ] The popup appears on the active Space and over a full-screen app
- [ ] With no network, the popup reports an offline failure specifically rather than a generic error
