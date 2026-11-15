import { describe, expect, it } from "vitest";

import { requestIdFor, tokenForRequestId } from "./ipc";

describe("the Improve request id on the wire", () => {
  it("round-trips a token through the string the shell echoes back", () => {
    const token = { session: 4, sequence: 12 };

    expect(tokenForRequestId(requestIdFor(token))).toEqual(token);
  });

  it("keeps attempts from different sessions apart even when their sequence matches", () => {
    const first = requestIdFor({ session: 1, sequence: 1 });
    const reopened = requestIdFor({ session: 2, sequence: 1 });

    expect(first).not.toBe(reopened);
    expect(tokenForRequestId(reopened)).toEqual({ session: 2, sequence: 1 });
  });

  it("treats an id this build did not mint as no answer at all", () => {
    expect(tokenForRequestId("not-a-token")).toBeNull();
  });
});
