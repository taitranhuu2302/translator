# 04: Answer token — one owner for "the newest answer wins"

**What to build:** One small module owns freshness for every asynchronous answer. It issues a token when an attempt starts and answers whether that token is still current. A token carries both a session identity and a sequence number, so it survives the case a bare counter cannot: the Quick Translate popup is reopened, a new session starts translating, and the previous session's answer arrives afterwards. The popup, the Translate Window's Translate pane, and its Improve pane all store a token instead of inventing their own counter, and the wire's string request id is converted in the IPC module only.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] The token module issues and retires tokens; no other module defines its own request counter.
- [ ] The popup reducer drops an answer from a session that is no longer current, including the reopen-while-translating case the current test does not cover.
- [ ] The Translate and Improve reducers accept an answer or a chunk only for a current token; a restored history entry stays valid from any phase because it answers no request.
- [ ] The IPC module is the only place a token becomes the wire's request id and back.
- [ ] The component-level session counter disappears; the reducer's own rule is sufficient on its own.
- [ ] Tests cover: a late answer from a previous session, a late chunk from an abandoned attempt, a current answer accepted, and two sequential sessions in the same open popup.
