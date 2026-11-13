/**
 * The provider section's pure rules. The component only draws them, so what the
 * status means is tested without a window or a Keychain.
 */
import type { ProviderStatus, Settings } from "./ipc";

/** One line saying whether Azure will be used, and why not when it will not. */
export function azureStatusLine(status: ProviderStatus): string {
  if (status.azure_skipped) {
    return "Skipped for this session after an authentication or quota failure. A new key is tried again.";
  }
  if (status.azure_configured) {
    return "Key saved in the Keychain. Azure answers first, with MyMemory as the fallback.";
  }
  return "No key yet. MyMemory translates without one, so everything still works.";
}

/** One line saying whether Groq — Improve's model — will be used. Improve has
 * no keyless fallback, so this is the one that has to name the key. */
export function groqStatusLine(status: ProviderStatus): string {
  if (status.groq_skipped) {
    return "Skipped for this session after an authentication or quota failure. A new key is tried again.";
  }
  if (status.groq_configured) {
    return "Key saved in the Keychain. Improve uses Groq; translation never waits on it.";
  }
  return "No key yet. Improve needs a Groq key; translation works without one.";
}

/** MyMemory is the keyless fallback, so its row only ever says it is there. */
export function myMemoryStatusLine(): string {
  return "No key needed — MyMemory is always available, and translates when Azure is missing or skipped.";
}

/** The region is stored trimmed: whitespace must not become a header value. */
export function withAzureRegion(settings: Settings, region: string): Settings {
  return { ...settings, azure_region: region.trim() };
}

/** Save is enabled only when the region actually changed. */
export function regionChanged(settings: Settings, region: string): boolean {
  return region.trim() !== settings.azure_region.trim();
}
