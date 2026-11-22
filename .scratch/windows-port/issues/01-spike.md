# 01: Spike — a non-activating popup and UI Automation on Windows

**Type:** prototype

**What to build:** A throwaway Tauri v2 window on a Windows machine that proves
the two things the port cannot assume: that a popup can be shown without
activating the app while still receiving Enter and Escape, and that UI
Automation's `TextPattern` yields the Selection in the applications people
actually use — Chrome, an Electron app, Windows Terminal, Notepad, Word. The
spike is not imported into the tree; its answer is.

The macOS answer to the same question is ADR-0005, including the fallback it
named. This ticket decides whether the same architecture holds or whether the
popup has to be drawn natively.

**Blocked by:** None (needs a Windows machine, which the agent does not have)

**Status:** open

- [ ] A window with `WS_EX_NOACTIVATE` (or the Tauri equivalent) shows above a
      full-screen app without taking focus; the previously focused app keeps
      its caret and selection.
- [ ] With that window key, Enter and Escape reach the web view.
- [ ] `TextPattern` returns the selected text in at least Chrome, one Electron
      app, Windows Terminal and Notepad; the answer is recorded per app.
- [ ] The clipboard round trip (synthesized Ctrl+C, restore afterwards) covers
      the apps `TextPattern` does not.
- [ ] Secure/ password fields are refused, not silently read.
- [ ] The answer is written into this ticket, and the port's tickets are
      adjusted to it before any of them start.
