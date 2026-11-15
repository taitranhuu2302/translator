/**
 * One owner for "the newest answer wins".
 *
 * Every asynchronous answer — a translation, a streamed Improve chunk, a typed
 * failure — is tagged with the token issued when its attempt started. A surface
 * stores the token it is waiting on and drops anything that is not that token.
 *
 * The token carries a session identity as well as a sequence number because a
 * bare counter is not enough: the Quick Translate popup is reopened, a new
 * session starts, and the previous session's answer arrives afterwards. Its
 * sequence can match the new session's, so only the session identity tells the
 * two attempts apart.
 */

/** One attempt's identity. `session` names the open surface it belongs to and
 * `sequence` counts attempts within that session. */
export interface AnswerToken {
  readonly session: number;
  readonly sequence: number;
}

/** A sequence of attempts made by one open surface. Reopening a surface starts
 * a new session, retiring every token the previous one issued. */
export interface AnswerSession {
  /** Issue the token for a new attempt, retiring the attempt before it. */
  issue(): AnswerToken;
}

let nextSession = 0;

/** Open a new session. No token from an earlier session is ever current again. */
export function beginSession(): AnswerSession {
  const session = ++nextSession;
  let sequence = 0;
  return {
    issue() {
      sequence += 1;
      return { session, sequence };
    },
  };
}

/** Whether `token` is the attempt the surface is still waiting on. An answer
 * with no current token — because none is in flight, or the surface was
 * cleared or dismissed — is never current. */
export function isCurrent(
  token: AnswerToken | null,
  current: AnswerToken | null,
): boolean {
  return (
    token !== null &&
    current !== null &&
    token.session === current.session &&
    token.sequence === current.sequence
  );
}
