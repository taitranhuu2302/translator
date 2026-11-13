# Manual checks — 08: Improve

Everything below needs a human at the machine. The suites prove the SSE
parsing at every byte boundary, the incremental JSON scanner, the provider's
request shape and error mapping, the request-id reducer, and the engine seam;
only the running web view can show that the two panes fill as the model writes,
that each Copy button carries only its own text, and that the real Groq endpoint
and Keychain wiring behave as specified in `groq.rs` and `App.tsx`.
`./scripts/build.sh` must have produced a clean, signed
`target/bundle/Translator.app` first. The app was **never launched** while
building this ticket.

Open the window with ⌃⌥E, from the menu bar item (**Open Translate Window**),
then click **Improve**. Improve needs a Groq key: open **Provider keys**, paste
one, and click **Save key**.

## Reset between runs

Quit the app from the menu bar (**Quit Translator**). Then remove the state you
want to start fresh:

```bash
# Forget the remembered window frame.
rm -f "$HOME/Library/Application Support/com.taitran.translator/window-frame.json"

# Or start from default settings as well (also forgets the Language Pair).
rm -f "$HOME/Library/Application Support/com.taitran.translator/settings.json"

# Forget the Groq key (start keyless).
security delete-generic-password -s com.taitran.translator -a groq 2>/dev/null || true
```

To watch what the app logs while testing:

```bash
log stream --style compact --predicate 'process == "Translator"'
```

To prove a keyless Improve opens no connection to Groq:

```bash
sudo tcpdump -n -i any host api.groq.com
```

## Cases

| # | Case | Steps | Expected |
| --- | --- | --- | --- |
| 1 | The mode switch changes the pane | Open the window, click **Improve**. | The source hint says Enter improves, the action button reads **Improve**, and the result pane shows a **Corrected** section above a **Better** section. There is no **Swap** button. |
| 2 | The mode switch keeps the typed text | Type `me and him goes home`, switch to **Translate**, then back to **Improve**. | The source text is still exactly `me and him goes home`; each mode keeps its own result pane. |
| 3 | Improve streams visibly | Throttle the network (Network Link Conditioner) and press **Enter** on a two-sentence paragraph. | Both sections appear immediately as labels and fill progressively — words arrive in both before the request finishes. The window never freezes and the input stays editable. |
| 4 | Shift+Enter adds a line | Type `He go home`, press **Shift+Enter**, type `She are late`, press **Enter**. | Shift+Enter inserts a newline instead of improving. Enter then improves the whole two-line sentence. |
| 5 | The corrected part preserves meaning and register | Improve a casual sentence (`me and him goes to the store yesterday`) and a formal one (`The committee have reached a decision`). | Each corrected version fixes the grammar while keeping the author's meaning and register — the casual one stays casual, the formal one stays formal. It is not rewritten into a different voice. |
| 6 | The better part is more natural or concise | Compare **Better** with **Corrected** for both sentences in case 5. | **Better** reads more naturally or says the same thing more concisely, still faithful to the original. It is not a summary, a translation, or unrelated prose. |
| 7 | The two parts are separate, never prose to split | Look at the pane after an improvement. | Two labelled sections with distinct text. No part of the model's answer is shown as unlabelled prose, and the two answers are never joined into one paragraph. |
| 8 | Copy copies only the Corrected part | Click **Copy** under **Corrected**, then paste into another app. | The clipboard holds exactly the corrected sentence — no better sentence, no label, no quotes. Only that button reads **Copied**. |
| 9 | Copy copies only the Better part | Click **Copy** under **Better**, then paste elsewhere. | The clipboard holds exactly the better sentence. The Corrected button no longer reads **Copied**; only Better does. |
| 10 | Empty input clears both parts and sends nothing | With an improvement showing, select all the input and delete it, then press **Enter**. | The result pane returns to its empty prompt with no sections, no error, and no request. |
| 11 | Whitespace only is empty | Type only spaces or newlines and press **Enter**. | Same as case 10: both sections clear without a request. |
| 12 | Switching modes mid-stream does not cross the panes | Start an Improve on a long paragraph, switch to **Translate** while the sections are still filling, and translate something. Switch back to **Improve**. | The Improve stream keeps filling its own sections in the background and the Translate result never appears in the Improve pane. Neither pane shows the other's text. |
| 13 | A missing key names Groq | With no Groq key saved, press **Enter** on `He go home`. | The pane says **Groq needs an API key.** and points at Provider keys. It does not say "No translation provider is available" and does not fail silently. The source text is intact and editable. |
| 14 | A keyless Improve asks no provider and records nothing | With no Groq key, run `sudo tcpdump -n -i any host api.groq.com` and press **Enter**. | tcpdump shows no connection to `api.groq.com`. Until the history ticket lands, the automated proof that nothing is written is `improve_is_not_recorded_in_history_until_the_history_ticket_owns_it`. |
| 15 | The Groq status line names the key | Open **Provider keys**. | The **Groq (Improve)** section says **No key** with "No key yet. Improve needs a Groq key; translation works without one." after saving it says **Key saved**, and **Clear key** removes it and the line returns to the keyless wording. |
| 16 | Malformed model output is a typed error, not a guessed answer | Run the stub below in one terminal, temporarily change `ENDPOINT` in `crates/core/src/providers/groq.rs` to `http://127.0.0.1:8765/`, rebuild with `./scripts/build.sh`, and press **Improve**. | The stub's document ends mid-string, so the strict final parse fails: the pane shows "The translation service sent something unexpected." No half answer is displayed as the result, and the input is untouched. Revert the constant and rebuild afterwards. |
| 17 | A truncated model answer is an error | Improve a very long paragraph so the reply hits the provider's token limit (`finish_reason: "length"`). | The pane shows a typed failure instead of an incomplete pair; the input is untouched. (Non-deterministic; record what the model actually returned.) |
| 18 | A network failure keeps the input | Turn Wi-Fi off, press **Enter** on a sentence. Then turn Wi-Fi on and press **Enter** again. | The pane shows the typed offline/network failure; the source text is unchanged and editable. The second Enter retries and fills both sections. |
| 19 | The sections scroll independently | Improve a long paragraph whose Better version is much shorter than the Corrected one. | Corrected scrolls inside its own section; Better keeps its own space and the pane does not merge or overlap the two. |
| 20 | A fresh opening starts in Translate | After improving, close the window, select text in another app and press ⌃⌥E. | The window opens in **Translate** with the Selection in the input; the Improve sections from the previous window are gone. |

For case 16, the stub:

```bash
cat > /tmp/broken-groq.py <<'PY'
from http.server import BaseHTTPRequestHandler, HTTPServer

EVENTS = [
    b'data: {"choices":[{"delta":{"content":"{\\"corrected\\":\\"He goes home.\\","},"finish_reason":null}]}\n\n',
    b'data: {"choices":[{"delta":{},"finish_reason":"stop"}]}\n\n',
    b'data: [DONE]\n\n',
]

class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        self.rfile.read(int(self.headers.get("Content-Length", 0)))
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        for event in EVENTS:
            self.wfile.write(event)
            self.wfile.flush()

    def log_message(self, *args):
        pass

HTTPServer(("127.0.0.1", 8765), Handler).serve_forever()
PY
python3 /tmp/broken-groq.py
```

Case 3 needs a throttled network and case 17 depends on the model hitting its
token limit; if either cannot be arranged, record that in the results rather
than marking it passed.

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
