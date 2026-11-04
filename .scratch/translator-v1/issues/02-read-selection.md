# 02: Read the Selection

**What to build:** The app can see what the user has selected in any app. Selecting a paragraph in Notes, page text in Chrome, a line in VS Code or Terminal, then opening the Translate Window shows that text already sitting in the input pane. This is the first slice where a real user gets a result from a real document, and it carries the two failure paths the rest of the app depends on: reading through the clipboard when Accessibility exposes nothing, and refusing to read at all when the focused field is secure.

**Blocked by:** 01

**Status:** ready-for-agent

- [ ] With the Translate Window opened from a Selection, the input pane holds that Selection, in Notes, Chrome page text, VS Code and Terminal
- [ ] When Accessibility is not granted, the app says why it needs it and opens the right settings pane; after granting, the same flow works without reinstalling
- [ ] Reading through the clipboard restores the user's previous clipboard contents afterwards
- [ ] With a password field or secure event input active, the app refuses to read and no selected text leaves the machine
- [ ] With nothing selected, the app opens with an empty input pane and reports no error
- [ ] The onboarding that appears on first launch explains the four shortcuts and what each does
