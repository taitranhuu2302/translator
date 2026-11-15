import type { ProviderId } from "./ipc";

const PROVIDER_NAMES: Record<ProviderId, string> = {
  azure: "Azure",
  my_memory: "MyMemory",
  groq: "Groq",
};

/** The provider's name as the UI writes it. */
export function providerLabel(provider: ProviderId): string {
  return PROVIDER_NAMES[provider];
}

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
