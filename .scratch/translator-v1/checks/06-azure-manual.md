# Manual checks — 06: Azure F0 and the provider ladder

Everything below needs a human at the machine and a real Azure Translator
resource. The test suites prove the request shape, every status mapping, the
session skip and the wording, but only a real Keychain, a real Azure subscription
and the signed bundle can show that the wiring in `main.rs` is right.
`./scripts/build.sh` must have produced a clean, signed
`target/bundle/Translator.app` first.

Azure is the first rung and MyMemory the second. With no key the app must look
exactly as it did before this ticket: MyMemory answers and nothing is shown. With
a key, Azure answers and the popup names it. A bad key or an exhausted quota
falls back to MyMemory for the rest of the session, without an error.

## Reset between runs

Quit the app from the menu bar (**Quit Translator**). Then choose the state you
want:

```bash
# Remove the Azure key, leaving the rest of the settings alone.
security delete-generic-password -s com.taitran.translator -a azure 2>/dev/null

# Or start from default settings as well.
rm -f "$HOME/Library/Application Support/com.taitran.translator/settings.json"
```

To confirm what the Keychain holds without printing the secret:

```bash
security find-generic-password -s com.taitran.translator -a azure
```

Exit status 0 means an item exists; `errSecItemNotFound` means it does not. Do
not run `-w`: this check must never put the key on screen or in shell history.

To see what the app logs, run this in another terminal while testing:

```bash
log stream --style compact --predicate 'process == "Translator"'
```

## Cases

| # | Case | Steps | Expected |
| --- | --- | --- | --- |
| 1 | No key stays keyless, with no error | Delete the Azure item (reset above). Launch, open **Provider keys**, and translate a Vietnamese sentence with ⌃⌥D. | The section says **No key** and explains that MyMemory translates without one. The popup shows the translation and names **MyMemory**. No error, no dialog, and no request to Azure is made. |
| 2 | A valid key makes Azure answer | In **Provider keys**, paste a valid Azure Translator key, press **Save key**. Translate a sentence. | The badge changes to **Key saved**. The popup shows the translation and names **Azure** in its header. `security find-generic-password -s com.taitran.translator -a azure` now exits 0. |
| 3 | The key survives a restart | Quit and relaunch the app, then translate again. | **Provider keys** still says **Key saved** and the popup still names **Azure**. Nothing had to be re-entered. |
| 4 | The key never lands in the settings file or the log | After case 2, run: `grep -i "<first 6 characters of the key>" "$HOME/Library/Application Support/com.taitran.translator/settings.json"` and review the `log stream` output from all of cases 2–3. | The settings file has no key field and no match. The log stream contains no key material, no `Ocp-Apim-Subscription-Key` value, and no body containing the key. The settings file contains only `language_pair`, `shortcuts`, `start_at_login`, `azure_region` and `onboarding_seen`. |
| 5 | A bad key falls back to MyMemory once, then is never retried | Replace the saved key with a deliberately wrong one (**Save key**), then translate the same sentence twice. Open **Provider keys** after each try. | The first translation falls back to MyMemory and the popup names **MyMemory**. The second also names **MyMemory** with no error and no waiting on Azure. After the first failure the section reads **Skipped for this session after an authentication or quota failure.** Automated proof that Azure is not called again: `an_authentication_failure_is_never_retried_that_session`. |
| 6 | An exhausted quota behaves the same way | If a quota-exhausted key is available (or the F0 monthly quota has run out), save it and translate twice. | Both translations come from MyMemory, named in the popup, with no quota dialog shown to the user. The section says the provider is **skipped for this session**. |
| 7 | A 403/403000 points at the resource, not at billing | Use a key from a resource that is disabled, deleted, or in a different region than the saved region, then translate. | The app falls back to MyMemory and the popup still translates. If the failure is shown at all (for example with MyMemory also unavailable), the message says **Azure refused the request for this resource. Check the Azure resource: it may be disabled, suspended, or in another region.** It must not mention quota or ask for a new key. The vitest `a refused Azure resource points at the resource, never at billing quota` pins this wording. |
| 8 | The region is respected | With a **regional** resource: save its region (e.g. `westeurope`) and translate; then clear the region (**Save region**) and translate. With a **global** resource: enter a region and translate, then clear it. | Regional resource: translations name Azure with the region saved; clearing it makes Azure fail (401/403) and the app falls back to MyMemory, showing the failure only if both providers fail. Global resource: entering a region makes Azure reject the request; clearing it restores Azure answers. Either way the next translation after saving the region uses it without a restart. |
| 9 | Clearing the key returns to keyless | With a working key, press **Clear key**. Translate. Quit and relaunch, then translate again. | The badge returns to **No key**, the popup names MyMemory, and no error appears. The key is still absent after the relaunch. |
| 10 | Saving a key clears a session skip | With a bad key from case 5, enter a valid key and press **Save key**, then translate without restarting. | The **skipped for this session** line disappears immediately and the popup names **Azure** again. The app did not need a relaunch. |
| 11 | Offline is named as offline | Turn Wi-Fi off, translate with a valid key. | Both rungs fail; the popup says the machine appears to be offline and asks for a network connection. Nothing is fabricated. Turn Wi-Fi on and repeat: Azure answers again. |
| 12 | Replace reports the same provider wording | With Wi-Fi off, select text and press ⌃⌥R. | The **Replace** dialog names the offline failure and leaves the document unchanged. With a working provider, Replace writes the translation and no dialog appears. |
| 13 | The skip is per session, not permanent | After case 5, quit and relaunch with the bad key still saved. | The first translation tries Azure again, fails, falls back to MyMemory, and the section shows it skipped again. A skip never survives a restart; only a saved key does. |

Case 6 needs a quota-exhausted key and case 7 needs a resource in a broken state;
if neither can be arranged, record that in the results rather than marking them
passed.

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
