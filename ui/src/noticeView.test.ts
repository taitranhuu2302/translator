import { describe, expect, it } from "vitest";

import type { Notice } from "./notice";
import { CaptureNoticeView, NoticeView } from "./NoticeView";

/** Every piece of text a rendered element would show, in order. */
function textOf(node: unknown): string[] {
  if (node === null || node === undefined || node === false) return [];
  if (typeof node === "string" || typeof node === "number") return [String(node)];
  if (Array.isArray(node)) return node.flatMap(textOf);
  if (typeof node === "object" && "props" in node) {
    const element = node as {
      type?: unknown;
      props: { children?: unknown };
    };
    // A view may return another view; follow it to the host element it draws.
    if (typeof element.type === "function") {
      return textOf(
        (element.type as (props: typeof element.props) => unknown)(
          element.props,
        ),
      );
    }
    return textOf(element.props.children);
  }
  return [];
}

const notice: Notice = {
  kind: "capture",
  title: "Neris needs Accessibility to read your selection.",
  detail: "Open System Settings and enable Neris.",
  action: { id: "open_accessibility_settings", label: "Open System Settings" },
};

describe("the notice view", () => {
  it("draws the model's title, detail and action label and nothing else", () => {
    const element = NoticeView({ notice, className: "notice" });

    expect(textOf(element)).toEqual([
      notice.title,
      notice.detail,
      notice.action?.label,
    ]);
  });

  it("adds no action when the model carries none", () => {
    const quiet: Notice = {
      kind: "capture",
      title: "This is a secure field, so nothing was read.",
      detail: "Passwords stay on your Mac.",
    };

    const element = NoticeView({ notice: quiet, className: "popup-failure" });

    expect(textOf(element)).toEqual([quiet.title, quiet.detail]);
  });

  it("a surface draws the model and adds no wording of its own", () => {
    const element = CaptureNoticeView({
      notice,
      granted: false,
      shortcut: "⌘⇧T",
    });

    expect(textOf(element)).toEqual([
      notice.title,
      notice.detail,
      notice.action?.label,
    ]);
  });

  it("replaces a resolved Accessibility failure with the model's granted notice", () => {
    const element = CaptureNoticeView({
      notice,
      granted: true,
      shortcut: "⌘⇧T",
    });

    expect(textOf(element)).toEqual([
      "Accessibility is on.",
      "Select text in any app and press ⌘⇧T again.",
    ]);
  });
});
