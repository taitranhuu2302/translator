import { describe, expect, it } from "vitest";

import type { CaptureError, EngineError } from "./ipc";
import {
  accessibilityGrantedNotice,
  isAccessibilityNotice,
  noticeFor,
  noticeForCapture,
  type NoticeKind,
} from "./notice";

const KINDS: NoticeKind[] = [
  "offline",
  "capture",
  "provider",
  "error",
  "ok",
];

const CAPTURE_CASES: CaptureError[] = [
  "no_accessibility",
  "secure_input",
  "nothing_selected",
  "nothing_to_replace",
  "not_writable",
  { unavailable: { reason: "the Accessibility API returned no selection" } },
];

/** One of every EngineError variant the wire can deliver. */
const ENGINE_ERRORS: EngineError[] = [
  "nothing_to_translate",
  ...CAPTURE_CASES.map((capture) => ({ capture })),
  { provider: "not_configured" },
  { provider: "offline" },
  { provider: { missing_key: { provider: "azure" } } },
  { provider: { quota_exceeded: { provider: "azure" } } },
  { provider: { authentication_failed: { provider: "azure" } } },
  {
    provider: {
      authentication_failed: {
        provider: "azure",
        cause: "resource_not_authorized",
      },
    },
  },
  { provider: { network: { provider: "azure", message: "timed out" } } },
  {
    provider: {
      malformed_response: { provider: "azure", message: "not JSON" },
    },
  },
  { provider: { unavailable: { reason: "maintenance" } } },
  { speech: { no_voice: { language: "fr" } } },
  { speech: { unavailable: { reason: "no voices" } } },
  { history: { unavailable: { reason: "disk" } } },
  { history: { storage: { message: "locked" } } },
  { settings: { unavailable: { reason: "disk" } } },
  { settings: { storage: { message: "locked" } } },
  { credentials: { unavailable: { reason: "keychain" } } },
  { credentials: { storage: { message: "locked" } } },
];

describe("the capture notice", () => {
  it("settles on one Accessibility text and one action for both surfaces", () => {
    const notice = noticeForCapture("no_accessibility");

    expect(notice.kind).toBe("capture");
    expect(notice.title).toBe(
      "Neris needs Accessibility to read your selection.",
    );
    expect(notice.detail).toBe(
      "Open System Settings → Privacy & Security → Accessibility, enable Neris, then press the shortcut again.",
    );
    expect(notice.action).toEqual({
      id: "open_accessibility_settings",
      label: "Open System Settings",
    });
  });

  it("settles on one secure-input text", () => {
    const notice = noticeForCapture("secure_input");

    expect(notice.title).toBe("This is a secure field, so nothing was read.");
    expect(notice.detail).toBe(
      "Passwords stay on your Mac. Finish there, then try again elsewhere.",
    );
    expect(notice.action).toBeUndefined();
  });

  it("settles on one fallback text for a capture failure it does not know", () => {
    const notice = noticeForCapture("brand_new" as unknown as CaptureError);

    expect(notice.title).toBe("The selection could not be read.");
    expect(notice.detail).toBe("Try again.");
    expect(notice.action).toBeUndefined();
  });

  it("words nothing selected without telling the user to select nothing", () => {
    const notice = noticeForCapture("nothing_selected");

    expect(notice.title).toBe("Select something first.");
    expect(notice.detail).toBe(
      "Select text in any app, then press the shortcut again.",
    );
  });

  it("words the resolved Accessibility state without an action", () => {
    const notice = accessibilityGrantedNotice("⌘⇧T");

    expect(notice.kind).toBe("ok");
    expect(notice.title).toBe("Accessibility is on.");
    expect(notice.detail).toBe(
      "Select text in any app and press ⌘⇧T again.",
    );
    expect(notice.action).toBeUndefined();
  });

  it("knows which notice the Accessibility grant resolves", () => {
    expect(isAccessibilityNotice(noticeForCapture("no_accessibility"))).toBe(
      true,
    );
    expect(isAccessibilityNotice(noticeForCapture("secure_input"))).toBe(false);
  });

  it("passes an unavailable reason through as the detail", () => {
    const notice = noticeForCapture({
      unavailable: { reason: "the selected text was not valid UTF-8" },
    });

    expect(notice.detail).toBe("the selected text was not valid UTF-8");
  });

  it("offers no action except the Accessibility fix", () => {
    for (const capture of CAPTURE_CASES) {
      const notice = noticeForCapture(capture);
      if (capture === "no_accessibility") {
        expect(notice.action).toBeDefined();
      } else {
        expect(notice.action).toBeUndefined();
      }
    }
  });
});

describe("the engine notice", () => {
  it.each(ENGINE_ERRORS)("words %j exactly once", (error) => {
    const notice = noticeFor(error);

    expect(notice.title.trim().length).toBeGreaterThan(0);
    expect(notice.detail.trim().length).toBeGreaterThan(0);
    expect(KINDS).toContain(notice.kind);
    // At most one action: the model never carries a list of things to do.
    expect(
      notice.action === undefined || typeof notice.action.label === "string",
    ).toBe(true);
  });

  it.each(CAPTURE_CASES)("maps %j the same through both doors", (capture) => {
    expect(noticeFor({ capture })).toEqual(noticeForCapture(capture));
  });

  it("names an offline failure as offline", () => {
    const notice = noticeFor({ provider: "offline" });

    expect(notice.kind).toBe("offline");
    expect(notice.title.toLowerCase()).toContain("offline");
  });

  it("styles a non-provider engine failure as an error, not a provider fault", () => {
    const notice = noticeFor({ speech: { no_voice: { language: "fr" } } });

    expect(notice.kind).toBe("error");
  });

  it("words the blank rule once and offers no action", () => {
    const notice = noticeFor("nothing_to_translate");

    expect(notice.kind).toBe("capture");
    expect(notice.title).toBe("There is nothing to translate.");
    expect(notice.detail).toBe("Select or type some text, then try again.");
    expect(notice.action).toBeUndefined();
  });
});
