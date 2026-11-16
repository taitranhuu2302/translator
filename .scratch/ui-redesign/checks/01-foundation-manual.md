# Foundation — manual checks

Written 2026-11-16 for ticket 01. Run these on the signed bundle after the
foundation lands. The agent did not launch the app.

| # | Do | Expect |
| --- | --- | --- |
| 1 | With macOS in **Light** appearance, open the app. | Window is dark. No light-grey surface, no white input, no system-coloured text anywhere. |
| 2 | Look at any heading and body text. | Inter, not the system face: tighter apertures, the `cv` alternates visible in `a`, `g`, `l`. Nothing looks like San Francisco. |
| 3 | Paste `ắ ệ ỏ ữ ợ ằ ẩ ỹ Ắ Ệ Ỡ` into the source pane. | Every glyph belongs to the same face — same stroke weight, same x-height. None sits lower, thinner, or in a different style. |
| 4 | Hover the primary button. | The button visibly changes. The hover is not invisible. |
| 5 | Tab through the toolbar, both panes and a settings row. | Every focused control shows a ring in Aurora Blue. Nothing is focusable but invisible. |
| 6 | Turn on **Reduce Motion** in System Settings, then open History and Settings. | Both appear without sliding or fading. Nothing animates. |
| 7 | Scroll a long History list. | The scrollbar is thin and dark, matching the palette. No white gutter, no default Aqua scrollbar. |

If case 3 fails, the font subset is wrong: check that `vietnamese` (or
`latin-ext`) is imported, not `latin` alone.
