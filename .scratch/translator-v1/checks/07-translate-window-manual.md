# Manual checks — 07: Translate Window

Everything below needs a human at the machine. The suites prove direction,
the result-pane state machine, the settings document staying untouched, the
frame geometry, and the shared engine path; only the running web view can show
that the keyboard handling, the clipboard, the window frame and the real
provider wiring behave as specified in `main.rs` and `App.tsx`.
`./scripts/build.sh` must have produced a clean, signed
`target/bundle/Translator.app` first. The app was **never launched** while
building this ticket.

Open the window with ⌃⌥E, from the menu bar item (**Open Translate Window**),
or by selecting text first so the Selection arrives in the input.

## Reset between runs

Quit the app from the menu bar (**Quit Translator**). Then remove the state you
want to start fresh:

```bash
# Forget the remembered window frame.
rm -f "$HOME/Library/Application Support/com.taitran.translator/window-frame.json"

# Or start from default settings as well (also forgets the Language Pair).
rm -f "$HOME/Library/Application Support/com.taitran.translator/settings.json"
```

Inspect the two documents side by side; the frame must never appear in the
settings document:

```bash
cat "$HOME/Library/Application Support/com.taitran.translator/window-frame.json"
cat "$HOME/Library/Application Support/com.taitran.translator/settings.json"
```

To watch what the app logs while testing:

```bash
log stream --style compact --predicate 'process == "Translator"'
```

## Cases

| # | Case | Steps | Expected |
| --- | --- | --- | --- |
| 1 | Enter translates | Open the window, type `Xin chào`, press **Enter**. | The result pane fills with the English translation and names the provider (`via MyMemory`, or `via Azure` when a key is saved). The source pane still shows exactly `Xin chào` and keeps the caret. |
| 2 | Shift+Enter adds a line | Type `Xin chào`, press **Shift+Enter**, type `bạn`, press **Enter**. | Shift+Enter inserts a newline instead of translating. Enter then translates the two-line text as one request. |
| 3 | The Translate button does the same | Type text and click **Translate**. | The same result as pressing Enter. The button is visible without scrolling. |
| 4 | Empty input does nothing and clears the pane | With a translation showing, select all the input and delete it, then press **Enter** and click **Translate**. | The result pane returns to its empty prompt. No new answer appears, nothing is sent to a provider, and no error is shown. |
| 5 | Whitespace only is empty | Type only spaces or newlines and press **Enter**. | Same as case 4: the result pane clears and no request is made. |
| 6 | In-flight state, input stays editable | Throttle the network (Network Link Conditioner or a very long paragraph) and press **Enter**. While the pane says `Translating…`, keep typing in the input. | The input accepts typing throughout. The pane shows `Translating…` rather than a blank result, and the window never looks frozen. |
| 7 | The result is selectable in part | After a translation, drag-select a phrase inside the result and press ⌘C, then paste elsewhere. | Only the selected phrase is copied; the result pane is not a disabled or read-only-looking box. |
| 8 | Copy copies only the translation | Click **Copy**, then paste into another app. | The clipboard contains the translation text only — no source text, no provider label, no quotes. The button reads **Copied** until the next translate, swap or edit. |
| 9 | Unavailable actions are disabled | Look at the result pane while it is empty, translating, or showing a failure. | **Swap** and **Copy** are disabled in all three states. |
| 10 | Swap reverses this window only | With Target English and partner Vietnamese (the default), translate `Xin chào` into English. Click **Swap**. | The English result becomes the source, the pane shows the Vietnamese translation of it, and `via …` still names the provider. `settings.json` still holds the same `language_pair`: the saved Target did not move. |
| 11 | Swap is a toggle | Click **Swap** again. | The direction reverses back and the pane shows the English text again. |
| 12 | A fresh opening starts from the saved pair | After a Swap, close the window and reopen it with ⌃⌥E (or quit and relaunch). | The window translates in the saved direction again; the swap was for that window only. |
| 13 | The window remembers its frame | Move the window and resize it, wait a second, then quit and relaunch. Open it again. | It comes back at the same position and size. `window-frame.json` exists and contains `x`, `y`, `width`, `height`; `settings.json` contains none of those fields. |
| 14 | The frame is physical pixels | On a Retina display, repeat case 13 and compare the file with the window's size. | The stored `width`/`height` are the physical pixel size (twice the point size on a 2× display), which is what Tauri's `outer_size` reports; the restored window is the same apparent size. |
| 15 | A frame on a disconnected display falls back | Move the window to a second display, quit, unplug that display, relaunch and open the window. | The window appears on a connected display at the default position instead of off screen. Automated proof of the geometry: the `frame` unit tests. |
| 16 | Closing hides rather than destroys | Type text, close the window with its close control, reopen it with ⌃⌥E. | The window reopens with the same text and result, at the same frame. The app did not quit, and the menu bar icon stayed. |
| 17 | Failures keep the input | Turn Wi-Fi off, type a sentence, press **Enter**. Then turn Wi-Fi on and press **Enter** again. | The result pane shows the typed failure (offline, or the provider's reason) and names the provider where there is one; the source text is untouched and still editable. The second Enter retries and fills the pane. |
| 18 | Quick Translate and the window share the cache | Translate the same sentence with ⌃⌥D, then paste it into the window and translate it again. | The second answer appears without a provider round trip. Automated proof that the engine asks the provider once: `translating_the_same_text_twice_through_the_engine_asks_the_provider_once`. |
| 19 | History is recorded through the same path | Translate from the window, then use the history search once ticket 09 lands. | The entry exists with the source text, the translation and `Translate` as the mode. Until ticket 09, `translate_text_records_what_it_translated_in_history` is the proof. |
| 20 | A capture failure does not break the typed panes | Revoke Accessibility (or focus a secure field and press ⌃⌥E), then type in the window. | The notice at the top explains the capture failure; the source pane is still editable and Enter still translates typed text. |
| 21 | Long text does not break the layout | Paste several paragraphs and translate. | The result pane scrolls, the panes keep their side-by-side layout, and the action buttons stay visible. |
| 22 | The shortcuts act on the window's own Selection | Select a phrase in the result pane and press ⌃⌥D, then dismiss it. Select a phrase in the source pane and press ⌃⌥S. | The popup opens beside the cursor with the result-pane phrase translated, exactly as it would from another app, and the window keeps its selection and its input. Read Aloud speaks the source-pane phrase. No shortcut ever acts on Translator's own interface text. |

Case 14 needs a Retina display and case 15 needs a second display; if either
cannot be arranged, record that in the results rather than marking it passed.

## Results

Fill this in only from a real run on the signed bundle. Anything failing becomes
a bug ticket.

| # | Result | Notes |
| --- | --- | --- |
| 1 | | |
| 2 | | |
| 3 | | |
| 4 | | |
| 5 | | |
| 6 | | |
| 7 | | |
| 8 | | |
| 9 | | |
| 10 | | |
| 11 | | |
| 12 | | |
| 13 | | |
| 14 | | |
| 15 | | |
| 16 | | |
| 17 | | |
| 18 | | |
| 19 | | |
| 20 | | |
| 21 | | |
| 22 | | |
