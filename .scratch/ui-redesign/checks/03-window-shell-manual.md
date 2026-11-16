# Window shell — manual checks

Written 2026-11-16 for ticket 03. Run these on the signed bundle. The agent did
not launch the app.

| # | Do | Expect |
| --- | --- | --- |
| 1 | Open the Translate Window. | It opens at 960×640. There is no separate title bar: the traffic lights sit on the app's own toolbar row and nothing overlaps them. |
| 2 | Drag an empty part of the toolbar. | The window moves. |
| 3 | Drag starting on a toolbar button. | The window does not move; the button activates instead. |
| 4 | Drag the window to its minimum size. | Panes stay side by side, nothing clips, no horizontal scrollbar appears. The window will not go below 680×460. |
| 5 | Type something, then switch to Improve and back. | The mode switch moves; each mode keeps its own content. |
| 6 | Quit, relaunch, resize, quit, relaunch. | The window returns at the size you left it. |
| 7 | Hover each trailing icon button. | A tooltip names it; the icon is a Lucide outline, 16–24px, consistent stroke. |
| 8 | Read the footer. | It names the Translate shortcut and the Replace shortcut, and both match Settings. |
| 9 | Translate something and look at the result pane's header. | A small badge names the provider that answered (`via Azure` or whichever you have keys for). It sits beside the pane's label and stays on one line at the minimum width. |
