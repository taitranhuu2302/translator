# Provider key row — manual checks

Written 2026-11-15 for ticket 06. Run these on the signed bundle; the agent did
not launch the app. Provider keys live in the login Keychain (service
`com.taitran.translator`); the settings file is
`~/Library/Application Support/com.taitran.translator/settings.json`. Restore
the file with `chmod u+w` after each failure case.

| # | Do | Expect |
| --- | --- | --- |
| 1 | Open Settings → Providers on an install with no Azure or Groq key. | Azure and Groq both show **No key**, neither shows **Clear key**, the key fields say *Paste an Azure Translator key* / *Paste a Groq API key*, and MyMemory still says it is always available. |
| 2 | Start from a build that already stored an Azure key (or save one in case 3, quit, relaunch, then reopen Settings). | The Azure row reads **Key saved** at once, the status sentence says the key is saved in the Keychain, the placeholder says *Saved — enter a new key to replace it*, and **Clear key** is present. Groq keeps whatever state its own key has. |
| 3 | Paste a key in the Azure field and click **Save key**. Quit and relaunch. | The field empties, the badge becomes **Key saved**, **Clear key** appears, and the notice is **Azure key saved in the Keychain.** After relaunch the Azure row still reads **Key saved**. |
| 4 | Repeat case 3 in the Groq row. | The Groq row shows the same behaviour, with **Groq key saved in the Keychain.**; the Azure row's field, badge and notice do not change. |
| 5 | Click **Clear key** on each provider that has a key. | That row's field empties, its badge returns to **No key**, **Clear key** disappears, and the notices are **Azure key cleared. MyMemory translates without one.** / **Groq key cleared. Improve will name the key it needs.** The other row is untouched. |
| 6 | With the key field empty or whitespace-only, try **Save key**. | The button is disabled; no write happens and no notice appears. |
| 7 | Type `eastus` in Azure **Region (optional)** and click **Save region**. Quit and relaunch. | The field shows `eastus`, the notice is **Region saved: eastus.**, and the region survives the relaunch. |
| 8 | `chmod a-w "$HOME/Library/Application Support/com.taitran.translator/settings.json"`, type `westeurope` and click **Save region**, then restore permissions. | The field keeps the typed `westeurope`, nothing is mirrored into the saved settings, and the sentence is the same **The setting could not be saved.** as every other settings write, never a region-specific wording. |
| 9 | While a key save is in flight, try the row's other buttons. | All of that row's actions are disabled until the write settles; the other provider's row stays usable. This is timing-sensitive, so treat a failure to observe it as inconclusive rather than a defect, and note the machine's Keychain latency. |
| 10 | Force a Keychain refusal (for example, lock the login keychain and cancel the unlock prompt), then save a key. | The draft keeps the typed key, the shown status does not change, and the only sentence is **The setting could not be saved.** |

The automated tests cover save, clear, empty draft, busy, a failed save and a
failed clear for both providers, plus the Azure region and the configured state
read from a stored key; this file is only for the drawn layout and the real
Keychain.
