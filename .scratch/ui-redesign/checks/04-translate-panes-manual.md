# Translate and Improve panes — manual checks

Written 2026-11-16 for ticket 04. Run these on the signed bundle. The agent did
not launch the app. Default Translate shortcut: `⌃⌥T`.

| # | Do | Expect |
| --- | --- | --- |
| 1 | Open the Translate Window with nothing typed. | Two cards side by side, each with a small uppercase label, the source one ready for input. The result side invites input rather than showing an empty box. |
| 2 | Click into the source pane. | The whole card lights up — border and a soft ring — not just a hairline inside it. There is exactly one ring, not two. |
| 3 | Type a short sentence and press Enter. | The result pane shows placeholder-shaped lines ("skeleton") while waiting, then the answer. Nothing jumps: the pane is the same height in both states, and the button that is working shows a spinner. |
| 4 | Type a long paragraph and press Enter. | The answer fills its pane and scrolls **inside** the result pane only; the source pane does not scroll and the window does not. |
| 5 | Look at the result pane's header. | Copy and Swap are there, small and quiet, disabled when they would do nothing. They are not at the bottom of the pane. |
| 6 | Press Copy, then Swap. | Copy turns into a tick briefly; Swap moves the answer into the source and the source into the result, exactly as today. |
| 7 | Hover Move/Swap and press Tab to it. | Both the hover and the focus ring are visible. No button looks the same before and after you point at it. |
| 8 | Switch to Improve with the same text. | The same two cards with a different verb; the result pane's header says the corrected text is the output. Nothing about the frame changes. |
| 9 | Disconnect the network and translate. | The failure is an alert with an icon and a tint, and its words are the ones the app has always used. It does not shift the layout. |
| 10 | Shrink the window to its minimum. | Both panes stay whole; the "Enter translates · Shift+Enter adds a line" hint steps away rather than ending in an ellipsis. |

At the minimum width, the footer hint is expected to disappear — the keys are in
the footer. A half-sentence is the only failure.
