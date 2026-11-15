# 06: Provider key row — one module, two adapters

**What to build:** One module renders a provider's key row and owns save, clear, busy state, and the notice that follows. Its interface is the provider plus the row's state; the Azure configuration additionally carries the region. Azure and Groq become two configurations of the same module instead of four hand-copied handlers and two duplicated blocks of markup. Key writes go through ticket 05's settings write, so the row inherits the one mirror rule and the one failure sentence.

**Blocked by:** 05 (Settings write — build next, save, mirror)

**Status:** resolved

- [x] Azure and Groq rows are two configurations of one module; there is one save path and one clear path.
- [x] The row owns its own draft, busy state, and notice; the page holds no per-provider copies of them.
- [x] Key state is written through ticket 05's settings write, so it follows the same mirror and failure rules as every other setting.
- [x] The two status paragraphs and the duplicated markup are gone; a new provider is a configuration, not a copied block.
- [x] Tests drive save, clear, empty draft refused, busy, and a failed save for both providers through the same module.
- [x] The cleared state and the configured state read correctly for a provider whose key was stored before this change.

## Implementation

- `ui/src/providerKey.ts` owns the row. `createProviderKeyRow(config, effects, initial)`
  is a subscribable state machine with one `save`, one `clear` and one
  `saveRegion`; it owns the write-only draft, `busy`, the notice, and the
  status the shell last answered. `AZURE_ROW` and `GROQ_ROW` are two values of
  `ProviderKeyRowConfig`; only their words and Azure's optional `region` differ.
  `isConfigured` is the one place the status's two flags become a provider's
  answer. The empty-draft refusal ("Enter a key before saving.") lives here.
- `ui/src/ProviderKeyRow.tsx` is the one renderer. It subscribes to the row with
  `useSyncExternalStore` and draws the badge, status sentence, key field,
  optional region field, actions and notice. `ProviderSettings.tsx` is now the
  two configurations plus the unchanged MyMemory row; it holds no key draft,
  busy flag, notice, status or status error.
- Key writes go through ticket 05's `settingsWriter`: `providerKey.ts` receives
  the `SettingsWriter` as an injected effect, and the two new mutations are
  `save_provider_key` and `clear_provider_key`. A failed write therefore
  inherits `SAVE_FAILED` ("The setting could not be saved."), not a key-specific
  copy. The region save is the existing `azure_region` mutation, so it keeps the
  same mirror rule and failure sentence too.
- Deleted from the page: four copied handlers (`saveKey`, `clearKey`,
  `saveGroqKey`, `clearGroqKey`), `keyDraft`/`groqDraft`/`busy`/`notice`/
  `groqNotice`/`status`/`statusError`, the per-provider `describe` helper, both
  duplicated status paragraphs and both duplicated key blocks. `WithAzureRegion`
  is no longer called by a view.
- `ChangeOutcome` settled as the tagged mirror union the merger's note asked
  for: `mirror` is `{kind:"settings", settings}` or
  `{kind:"provider_status", status}`, or `null`. `SettingsWriter.change` is
  overloaded per mutation family, so a settings view receives a
  `SettingsChange`, a key row receives a `ProviderKeyChange`, and only a reset
  receives `ResetChange`. There is one implementation and one set of failure
  sentences behind all three. `refusals` moved off the common branches onto
  `ResetChange`, the only outcome that produces them.
- The existing four write sites were updated to the tagged mirror and re-ran
  green: Target, shortcut rebind, reset and Azure region.

## Verification

| Command | Result |
| --- | --- |
| `./scripts/test.sh` | exit 0 — typecheck, 11 UI files / 161 tests, production build, IPC boundary, `cargo test --workspace` (34 + 121 + 3 + 78 + 22 + 2), re-run after merging `spec/architecture-deepening` at ticket 02 |
| `npm --prefix ui run test -- src/providerKey.test.ts src/settingsWrite.test.ts` | 41 tests (23 + 18) after the merge: load/configured/cleared for both providers, save, clear, empty draft refused, failed save and failed clear, busy refusing a second write, Azure region save/failure, Groq having no region; the four existing mutations under the tagged mirror, provider key save and clear mirroring a status, and both failure sentences |
| `npm --prefix ui run typecheck` | exit 0 |

The component's layout, the disabled buttons and the badge are drawing only and
are not covered by a DOM test. Manual steps are in
[`../checks/06-provider-key-row-manual.md`](../checks/06-provider-key-row-manual.md).
The app was not launched.

## Comments

- 2026-11-15 — Settling `ChangeOutcome`: the success branch now carries a tagged
  `Mirror` (`settings` or `provider_status`) rather than a second boolean or a
  second field. A second field would have reintroduced the second mirror
  decision C3 removed; the tag says which state channel the one mirrored value
  belongs to. The writer's overloads narrow the return per mutation family, so
  no view branches on `kind`. `refusals` moved onto `ResetChange`.
- 2026-11-15 — Each row reads `provider_status` once on mount rather than the
  page reading it once for both. That keeps status, draft, busy and notice in
  the row, so the page holds no per-provider state; the cost is a second
  Keychain status read when Settings opens.
- 2026-11-15 — The pure row module is `providerKey.ts`, not
  `providerKeyRow.ts`: the component is `ProviderKeyRow.tsx`, and the two names
  differ only by case, which collides on a case-insensitive filesystem.
- 2026-11-15 — Merged `spec/architecture-deepening` (ticket 02 and 03) after the
  implementation; it merged cleanly and the suite went green again at the
  counts above. Ticket 03's `failure.rs` and `notice.ts` do not touch the key
  row or the write protocol.
