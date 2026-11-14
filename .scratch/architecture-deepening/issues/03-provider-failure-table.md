# 03: Provider failure table — one mapping for every rung

**What to build:** One module in the core owns every conversion from a transport or HTTP outcome into a provider error. Each rung supplies its policy as data — which statuses mean authentication, which provider codes map to which authentication cause or to quota, what its server-error family is, and its rate-limit sentence — instead of a fourth copied match. A rung's module keeps only what is genuinely its own: request shape, response decoding, and its code table. User-facing sentences live next to the code table that selects them, not inside the rungs.

Every current per-provider decision is preserved exactly and turned into data: Azure's quota versus resource-not-authorized split on 403 with its fallback, the differing server-error families across rungs, and MyMemory's body-level quota detection. This ticket changes no behaviour; it gives existing behaviour one home.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] Transport failures are mapped in one place for all rungs.
- [ ] Rate limits, authentication statuses, server errors, and unrecognised statuses are mapped through one table, with each rung's policy expressed as data.
- [ ] The three copied transport mappers are gone, and no rung module formats a user-facing sentence.
- [ ] Behaviour is provably unchanged: through the ladder, authentication and quota failures still skip the rung for the session, a rate limit still does not, and every rung's server-error family and message are what they were.
- [ ] Tests drive these cases through the ladder rather than through the mappers, so they survive a rewrite of the module's internals.
- [ ] No public interface of the core widens: the table is internal to the providers side.
