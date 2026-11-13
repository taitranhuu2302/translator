import { describe, expect, it } from "vitest";

import type { ProviderStatus, Settings } from "./ipc";
import { azureStatusLine, groqStatusLine, regionChanged, withAzureRegion } from "./providerStatus";

const status = (overrides: Partial<ProviderStatus> = {}): ProviderStatus => ({
  azure_configured: false,
  groq_configured: false,
  azure_skipped: false,
  groq_skipped: false,
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

describe("the provider settings status", () => {
  it("says a keyless install still translates instead of showing an error", () => {
    const line = azureStatusLine(status());

    expect(line.toLowerCase()).toContain("no key");
    expect(line).toContain("MyMemory");
    expect(line.toLowerCase()).not.toContain("error");
  });

  it("says a saved key makes Azure answer first", () => {
    const line = azureStatusLine(status({ azure_configured: true }));

    expect(line).toContain("Key saved");
    expect(line).toContain("Azure");
  });

  it("reports a skipped provider and says a new key is tried again", () => {
    const line = azureStatusLine(
      status({ azure_configured: true, azure_skipped: true }),
    );

    expect(line.toLowerCase()).toContain("skipped");
    expect(line.toLowerCase()).toContain("new key");
  });
});

describe("the Groq status line", () => {
  it("names Groq as the key Improve needs, without pretending translation broke", () => {
    const line = groqStatusLine(status());

    expect(line).toContain("Groq");
    expect(line.toLowerCase()).toContain("improve");
    expect(line).toContain("translation works without one");
  });

  it("says a saved key is used and that translation never waits on it", () => {
    const line = groqStatusLine(status({ groq_configured: true }));

    expect(line).toContain("Key saved");
    expect(line).toContain("Groq");
  });

  it("reports a skipped Groq and says a new key is tried again", () => {
    const line = groqStatusLine(status({ groq_configured: true, groq_skipped: true }));

    expect(line.toLowerCase()).toContain("skipped");
    expect(line.toLowerCase()).toContain("new key");
  });
});

describe("saving the Azure region", () => {
  it("trims the typed region so whitespace never becomes a header value", () => {
    expect(withAzureRegion(settings, "  westeurope\n").azure_region).toBe("westeurope");
  });

  it("keeps every other setting untouched", () => {
    const next = withAzureRegion(settings, "eastus");

    expect(next.language_pair).toEqual(settings.language_pair);
    expect(next.shortcuts).toEqual(settings.shortcuts);
    expect(next.start_at_login).toBe(settings.start_at_login);
    expect(next.onboarding_seen).toBe(settings.onboarding_seen);
  });

  it("says nothing changed when the trimmed region matches what is saved", () => {
    const saved = withAzureRegion(settings, "westeurope");

    expect(regionChanged(saved, " westeurope ")).toBe(false);
    expect(regionChanged(saved, "eastus")).toBe(true);
  });
});
