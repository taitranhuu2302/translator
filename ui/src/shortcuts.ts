/**
 * Press-to-record shortcut capture and the settings write a rebind makes.
 *
 * The capture reads `event.code`, the physical key, so the combination is the
 * same whatever the keyboard layout types. It does not decide whether a
 * combination is usable — the engine does, with one wording for every path.
 */
import type { Settings, ShortcutAction } from "./ipc";

/** The part of a keyboard event capture needs. React's KeyboardEvent fits it. */
export interface KeyPress {
  code: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
}

/** Named keys the engine's parser accepts and that make sense globally. */
const NAMED_KEYS = new Set([
  "Space",
  "Enter",
  "Escape",
  "Tab",
  "Backspace",
  "Delete",
  "Insert",
  "Home",
  "End",
  "PageUp",
  "PageDown",
  "ArrowUp",
  "ArrowDown",
  "ArrowLeft",
  "ArrowRight",
  "Minus",
  "Equal",
  "BracketLeft",
  "BracketRight",
  "Backslash",
  "Semicolon",
  "Quote",
  "Comma",
  "Period",
  "Slash",
  "Backquote",
]);

/**
 * The combination a press means, or null when the press is only a modifier or a
 * key the settings cannot store. Modifier order matches how the shell stores and
 * shows a binding, so a captured and a stored spelling are the same string.
 */
export function captureBinding(press: KeyPress): string | null {
  const key = keyToken(press.code);
  if (!key) return null;

  const parts: string[] = [];
  if (press.metaKey) parts.push("Cmd");
  if (press.ctrlKey) parts.push("Ctrl");
  if (press.altKey) parts.push("Alt");
  if (press.shiftKey) parts.push("Shift");
  parts.push(key);
  return parts.join("+");
}

/** `KeyD` → `D`, `Digit1` → `1`; function and named keys keep their name. */
function keyToken(code: string): string | null {
  const letter = /^Key([A-Z])$/.exec(code);
  if (letter) return letter[1];

  const digit = /^Digit([0-9])$/.exec(code);
  if (digit) return digit[1];

  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;

  return NAMED_KEYS.has(code) ? code : null;
}

/** A settings value with one action's binding replaced; everything else shared. */
export function withShortcut(
  settings: Settings,
  action: ShortcutAction,
  binding: string,
): Settings {
  return {
    ...settings,
    shortcuts: { ...settings.shortcuts, [action]: binding },
  };
}
