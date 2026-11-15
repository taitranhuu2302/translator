import { describe, expect, it } from "vitest";

import { beginSession, isCurrent } from "./answerToken";

describe("the answer token", () => {
  it("makes the newest attempt in a session current and retires the one before", () => {
    const session = beginSession();
    const first = session.issue();
    expect(isCurrent(first, first)).toBe(true);

    const second = session.issue();

    expect(isCurrent(second, second)).toBe(true);
    expect(isCurrent(first, second)).toBe(false);
  });

  it("gives every attempt in one session its own sequence", () => {
    const session = beginSession();

    const tokens = [session.issue(), session.issue(), session.issue()];

    expect(new Set(tokens.map((token) => `${token.session}/${token.sequence}`)).size).toBe(
      tokens.length,
    );
  });

  it("keeps a token from a reopened surface from ever being current again", () => {
    // Reopening starts a new session, so the sequence restarts. Only the
    // session identity keeps the previous surface's answer from matching.
    const previous = beginSession().issue();
    const reopened = beginSession().issue();

    expect(reopened.sequence).toBe(previous.sequence);
    expect(isCurrent(previous, reopened)).toBe(false);
    expect(isCurrent(reopened, previous)).toBe(false);
  });

  it("treats nothing as current when no attempt is in flight", () => {
    const token = beginSession().issue();

    expect(isCurrent(token, null)).toBe(false);
    expect(isCurrent(null, token)).toBe(false);
    expect(isCurrent(null, null)).toBe(false);
  });
});
