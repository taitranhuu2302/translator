# Auto-detected direction — manual checks

Run these on the signed bundle. The agent did not launch the app.

| # | Do | Expect |
| --- | --- | --- |
| 1 | Open the Translate Window and translate `Xin chào`. | The result header shows `Vietnamese → English` beside `via …`, and the translation is English. |
| 2 | Clear the input and translate `Hello there`. | The direction pill reads `English → Vietnamese`, and the result is Vietnamese. |
| 3 | Translate `toi di hoc`. | The direction pill reads `English → Vietnamese`. This is the accepted v1 boundary, not a bug. |
| 4 | Press Swap on the first answer. | The input becomes the English text, the pill reads `English → Vietnamese`, and the new result is Vietnamese. |
| 5 | After Swap, type a character and press Enter. | The direction is detected again from the edited text; it is not forced to the swapped direction. |
| 6 | Open History, reopen a row translated earlier. | The pill shows the direction that row was translated in, even if the saved Target has changed since. |
| 7 | Select `Xin chào` anywhere and press `⌃⌥D`. | The popup header shows `Vietnamese → English · Azure` (or the provider that answered). |
| 8 | Select `Hello` anywhere and press `⌃⌥D`. | The popup header shows `English → Vietnamese · …`. |
| 9 | Open Settings with English as Target. | The sentence reads `Translates both ways between Vietnamese and English.` |
| 10 | Switch the Target to Vietnamese. | The sentence reads `Translates both ways between English and Vietnamese.` |
| 11 | Improve an English sentence. | Improve's pane shows no direction pill; only translations name a Direction. |
