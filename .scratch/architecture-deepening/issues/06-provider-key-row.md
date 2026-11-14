# 06: Provider key row — one module, two adapters

**What to build:** One module renders a provider's key row and owns save, clear, busy state, and the notice that follows. Its interface is the provider plus the row's state; the Azure configuration additionally carries the region. Azure and Groq become two configurations of the same module instead of four hand-copied handlers and two duplicated blocks of markup. Key writes go through ticket 05's settings write, so the row inherits the one mirror rule and the one failure sentence.

**Blocked by:** 05 (Settings write — build next, save, mirror)

**Status:** ready-for-agent

- [ ] Azure and Groq rows are two configurations of one module; there is one save path and one clear path.
- [ ] The row owns its own draft, busy state, and notice; the page holds no per-provider copies of them.
- [ ] Key state is written through ticket 05's settings write, so it follows the same mirror and failure rules as every other setting.
- [ ] The two status paragraphs and the duplicated markup are gone; a new provider is a configuration, not a copied block.
- [ ] Tests drive save, clear, empty draft refused, busy, and a failed save for both providers through the same module.
- [ ] The cleared state and the configured state read correctly for a provider whose key was stored before this change.
