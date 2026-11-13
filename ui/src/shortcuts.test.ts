import { describe, expect, it } from "vitest";

import type { Settings } from "./ipc";
import { captureBinding, withShortcut, type KeyPress } from "./shortcuts";

const press = (code: string, overrides: Partial<KeyPress> = {}): KeyPress => ({
  code,
  metaKey: false,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  ...overrides,
});

const settings: Settings = {
  language_pair: { target: "en", partner: "vi" },
  shortcuts: {
    quick_translate: "Ctrl+Alt+D",
    replace: "Ctrl+Alt+R",
    read_aloud: "Ctrl+Alt+S",
    translate_window: "Ctrl+Alt+E",
  },
  start_at_login: false,
  azure_region: "",
  onboarding_seen: true,
};

describe("capturing a shortcut", () => {
  it("ignores a modifier pressed on its own", () => {
    expect(captureBinding(press("ControlLeft", { ctrlKey: true }))).toBeNull();
    expect(captureBinding(press("MetaRight", { metaKey: true }))).toBeNull();
    expect(captureBinding(press("AltLeft", { altKey: true }))).toBeNull();
    expect(captureBinding(press("ShiftLeft", { shiftKey: true }))).toBeNull();
  });

  it("reads a letter from the physical key, not from the character it types", () => {
    // `code` is stable across layouts: Ctrl+Alt+D stays D on a French keyboard
    // even though the character there is something else.
    expect(captureBinding(press("KeyD", { ctrlKey: true, altKey: true }))).toBe(
      "Ctrl+Alt+D",
    );
  });

  it("spells a digit without the Digit prefix", () => {
    expect(
      captureBinding(press("Digit1", { ctrlKey: true, altKey: true })),
    ).toBe("Ctrl+Alt+1");
  });

  it("keeps a function key as it is", () => {
    expect(captureBinding(press("F5", { metaKey: true, shiftKey: true }))).toBe(
      "Cmd+Shift+F5",
    );
  });

  it("spells modifiers the way the settings file stores them", () => {
    expect(
      captureBinding(
        press("KeyK", {
          metaKey: true,
          ctrlKey: true,
          altKey: true,
          shiftKey: true,
        }),
      ),
    ).toBe("Cmd+Ctrl+Alt+Shift+K");
  });

  it("captures a key with no modifier so the engine can explain the refusal", () => {
    // The capture does not enforce the modifier rule; the engine does, with one
    // wording shared by every path.
    expect(captureBinding(press("KeyK"))).toBe("K");
  });

  it("ignores a key the parser cannot store", () => {
    expect(
      captureBinding(press("MediaPlayPause", { ctrlKey: true })),
    ).toBeNull();
    expect(
      captureBinding(press("ControlLeft", { ctrlKey: true, altKey: true })),
    ).toBeNull();
  });
});

describe("writing a rebound shortcut back into the settings", () => {
  it("replaces only the chosen binding and shares the rest", () => {
    const next = withShortcut(settings, "replace", "Cmd+Shift+L");

    expect(next.shortcuts).toEqual({
      ...settings.shortcuts,
      replace: "Cmd+Shift+L",
    });
    expect(next.language_pair).toBe(settings.language_pair);
    expect(next.onboarding_seen).toBe(settings.onboarding_seen);
  });
});
