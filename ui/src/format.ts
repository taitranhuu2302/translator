import type { EngineError } from "./ipc";

const MODIFIER_GLYPHS: Record<string, string> = {
  ctrl: "⌃",
  control: "⌃",
  alt: "⌥",
  option: "⌥",
  shift: "⇧",
  cmd: "⌘",
  command: "⌘",
  meta: "⌘",
  super: "⌘",
};

/**
 * "Ctrl+Alt+E" → "⌃⌥E". Bindings are stored as display strings until ticket 10
 * lets the user rebind them; this is the one place they become macOS glyphs.
 */
export function formatShortcut(binding: string): string {
  return binding
    .split("+")
    .map((part) => part.trim())
    .filter((part) => part.length > 0)
    .map((part) => MODIFIER_GLYPHS[part.toLowerCase()] ?? part.toUpperCase())
    .join("");
}

/** Why the window has no Selection: a permission, secure input, or a reason. */
export type CaptureFailure =
  | { kind: "no_accessibility" }
  | { kind: "secure_input" }
  | { kind: "unavailable"; reason: string };

export function captureFailure(error: EngineError): CaptureFailure {
  if ("capture" in error) {
    const capture = error.capture;
    if (capture === "no_accessibility") {
      return { kind: "no_accessibility" };
    }
    if (capture === "secure_input") {
      return { kind: "secure_input" };
    }
    if (typeof capture === "object" && "unavailable" in capture) {
      return { kind: "unavailable", reason: capture.unavailable.reason };
    }
  }
  return { kind: "unavailable", reason: "The selection could not be read." };
}
