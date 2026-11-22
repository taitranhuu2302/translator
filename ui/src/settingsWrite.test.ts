import { describe, expect, it } from "vitest";

import type { ProviderId, ProviderStatus, Settings, ShortcutAction } from "./ipc";
import {
  createSettingsWriter,
  type ProviderKeyChange,
  type ResetChange,
  type SettingsChange,
  type SettingsWriteEffects,
} from "./settingsWrite";

/** What the engine holds before any change in these tests. Every write must
 * carry the fields this fixture does not touch, so a partial object fails. */
const saved: Settings = {
  language_pair: { target: "en", partner: "vi" },
  shortcuts: {
    quick_translate: "Ctrl+Alt+D",
    // Not the shipped binding: a reset must be visible in the mirrored value.
    replace: "Cmd+Shift+L",
    read_aloud: "Ctrl+Alt+S",
    translate_window: "Ctrl+Alt+E",
  },
  start_at_login: false,
  azure_region: "westeurope",
  onboarding_seen: true,
  auto_check_updates: true,
};

/** What a reset writes: the shipped shortcuts and pair, every other field kept. */
const shipped: Settings = {
  ...saved,
  shortcuts: {
    quick_translate: "Alt+Shift+D",
    replace: "Alt+Shift+R",
    read_aloud: "Alt+Shift+S",
    translate_window: "Alt+Shift+E",
  },
};

/** The status the keychain answers after a write. It is the whole status, not
 * one provider's flag: the row mirrors the object the shell returned. */
const stored: ProviderStatus = {
  azure_configured: true,
  groq_configured: true,
  azure_skipped: false,
  groq_skipped: false,
};

const cleared: ProviderStatus = {
  azure_configured: false,
  groq_configured: false,
  azure_skipped: false,
  groq_skipped: false,
};

interface Calls {
  saves: Settings[];
  rebinds: { action: ShortcutAction; binding: string }[];
  resets: number;
  savedKeys: { provider: ProviderId; key: string }[];
  clearedKeys: ProviderId[];
}

/** The writer under test, with effects that record what it asked the shell to
 * do. Tests replace one effect to make it fail or answer differently. */
function harness(overrides: Partial<SettingsWriteEffects> = {}): {
  writer: ReturnType<typeof createSettingsWriter>;
  calls: Calls;
} {
  const calls: Calls = {
    saves: [],
    rebinds: [],
    resets: 0,
    savedKeys: [],
    clearedKeys: [],
  };
  const effects: SettingsWriteEffects = {
    read: async () => saved,
    save: async (next) => {
      calls.saves.push(next);
    },
    rebind: async (action, binding) => {
      calls.rebinds.push({ action, binding });
      return { binding, problem: null };
    },
    reset: async () => {
      calls.resets += 1;
      return { settings: saved, bindings: [], save_problem: null };
    },
    saveProviderKey: async (provider, key) => {
      calls.savedKeys.push({ provider, key });
      return stored;
    },
    clearProviderKey: async (provider) => {
      calls.clearedKeys.push(provider);
      return cleared;
    },
    ...overrides,
  };
  return { writer: createSettingsWriter(effects), calls };
}

/** The settings a view may mirror after a settings-shaped change. */
function mirroredSettings(
  outcome: SettingsChange | ResetChange,
): Settings | null {
  return outcome.mirror === null ? null : outcome.mirror.settings;
}

/** The notice a view draws after a settings-shaped change. */
function settingsNotice(
  outcome: SettingsChange | ProviderKeyChange | ResetChange,
): string | null {
  return outcome.mirror === null ? null : outcome.notice;
}

/** The provider status a key row may mirror. */
function mirroredStatus(outcome: ProviderKeyChange): ProviderStatus | null {
  return outcome.mirror === null ? null : outcome.mirror.status;
}

describe("changing the Target", () => {
  it("saves the whole settings object and returns it to mirror", async () => {
    const { writer, calls } = harness();

    const outcome = await writer.change({ kind: "target", target: "vi" });

    const expected: Settings = {
      ...saved,
      language_pair: { target: "vi", partner: "en" },
    };
    expect(mirroredSettings(outcome)).toEqual(expected);
    expect(calls.saves).toEqual([expected]);
  });
});

describe("a settings read that fails", () => {
  it("does not attempt the change and says the settings could not be read", async () => {
    const { writer, calls } = harness({
      read: async () => {
        throw new Error("the engine refused");
      },
    });

    const outcome = await writer.change({ kind: "target", target: "vi" });

    expect(outcome.mirror).toBeNull();
    expect(outcome).toMatchObject({
      problem: "The settings could not be read.",
    });
    expect(calls.saves).toEqual([]);
  });
});

describe("a write that fails", () => {
  it("does not mirror the value and gives the one save-failure sentence", async () => {
    const { writer } = harness({
      save: async () => {
        throw new Error("the settings file refused");
      },
    });

    const outcome = await writer.change({ kind: "target", target: "vi" });

    expect(outcome.mirror).toBeNull();
    expect(outcome).toMatchObject({
      problem: "The setting could not be saved.",
    });
  });
});

describe("a mutation that changes nothing", () => {
  it("saves nothing and leaves the shown settings equal to the saved ones", async () => {
    const { writer, calls } = harness();

    const outcome = await writer.change({ kind: "target", target: "en" });

    expect(calls.saves).toEqual([]);
    expect(mirroredSettings(outcome)).toEqual(saved);
    expect(settingsNotice(outcome)).toBeNull();
  });
});

describe("changing the Azure region", () => {
  it("trims the region, saves the whole object, and returns it to mirror", async () => {
    const { writer, calls } = harness();

    const outcome = await writer.change({
      kind: "azure_region",
      region: "  eastus\n",
    });

    const expected: Settings = { ...saved, azure_region: "eastus" };
    expect(mirroredSettings(outcome)).toEqual(expected);
    expect(calls.saves).toEqual([expected]);
    expect(settingsNotice(outcome)).toBe("Region saved: eastus.");
  });

  it("does not mirror a region that could not be saved", async () => {
    const { writer } = harness({
      save: async () => {
        throw new Error("the settings file refused");
      },
    });

    const outcome = await writer.change({
      kind: "azure_region",
      region: "eastus",
    });

    expect(outcome.mirror).toBeNull();
    // The same class of failure as the Target's, so the same sentence.
    expect(outcome).toMatchObject({
      problem: "The setting could not be saved.",
    });
  });

  it("says nothing and saves nothing when the trimmed region is already saved", async () => {
    const { writer, calls } = harness();

    const outcome = await writer.change({
      kind: "azure_region",
      region: " westeurope ",
    });

    expect(calls.saves).toEqual([]);
    expect(mirroredSettings(outcome)).toEqual(saved);
    expect(settingsNotice(outcome)).toBeNull();
  });
});

describe("changing the automatic update check", () => {
  it("saves the whole settings object and says the checks are off", async () => {
    const { writer, calls } = harness();

    const outcome = await writer.change({
      kind: "auto_check_updates",
      enabled: false,
    });

    const expected: Settings = { ...saved, auto_check_updates: false };
    expect(mirroredSettings(outcome)).toEqual(expected);
    expect(calls.saves).toEqual([expected]);
    expect(settingsNotice(outcome)).toBe(
      "Automatic update checks turned off.",
    );
  });

  it("says nothing and saves nothing when the setting already has that value", async () => {
    const { writer, calls } = harness();

    const outcome = await writer.change({
      kind: "auto_check_updates",
      enabled: true,
    });

    expect(calls.saves).toEqual([]);
    expect(mirroredSettings(outcome)).toEqual(saved);
    expect(settingsNotice(outcome)).toBeNull();
  });

  it("does not mirror a setting that could not be saved", async () => {
    const { writer } = harness({
      save: async () => {
        throw new Error("the settings file refused");
      },
    });

    const outcome = await writer.change({
      kind: "auto_check_updates",
      enabled: false,
    });

    expect(outcome.mirror).toBeNull();
    expect(outcome).toMatchObject({
      problem: "The setting could not be saved.",
    });
  });
});

describe("rebinding a shortcut", () => {
  it("mirrors the binding the shell applied and saved, without saving again", async () => {
    const { writer, calls } = harness();

    const outcome = await writer.change({
      kind: "rebind",
      action: "replace",
      binding: "Cmd+Shift+W",
    });

    expect(calls.rebinds).toEqual([
      { action: "replace", binding: "Cmd+Shift+W" },
    ]);
    expect(calls.saves).toEqual([]);
    expect(mirroredSettings(outcome)).toEqual({
      ...saved,
      shortcuts: { ...saved.shortcuts, replace: "Cmd+Shift+W" },
    });
  });

  it("does not mirror a refusal, and keeps the engine's own sentence", async () => {
    const { writer } = harness({
      rebind: async (_action, binding) => ({
        binding,
        problem: "That shortcut is already used by another action.",
      }),
    });

    const outcome = await writer.change({
      kind: "rebind",
      action: "replace",
      binding: "Ctrl+Alt+D",
    });

    expect(outcome.mirror).toBeNull();
    expect(outcome).toMatchObject({
      problem: "That shortcut is already used by another action.",
    });
  });

  it("does not mirror when the shell could not attempt the rebind", async () => {
    const { writer } = harness({
      rebind: async () => {
        throw new Error("the command refused");
      },
    });

    const outcome = await writer.change({
      kind: "rebind",
      action: "replace",
      binding: "Cmd+Shift+L",
    });

    expect(outcome.mirror).toBeNull();
    expect(outcome).toMatchObject({
      problem: "The shortcut could not be changed.",
    });
  });
});

describe("saving a provider key", () => {
  it("returns the refreshed provider status to mirror, and writes no settings", async () => {
    const { writer, calls } = harness();

    const outcome = await writer.change({
      kind: "save_provider_key",
      provider: "azure",
      key: "secret",
    });

    expect(calls.savedKeys).toEqual([{ provider: "azure", key: "secret" }]);
    expect(calls.saves).toEqual([]);
    expect(mirroredStatus(outcome)).toEqual(stored);
    expect(settingsNotice(outcome)).toBeNull();
  });

  it("does not mirror the status when the keychain refused, and gives the one write-failure sentence", async () => {
    const { writer } = harness({
      saveProviderKey: async () => {
        throw new Error("the keychain refused");
      },
    });

    const outcome = await writer.change({
      kind: "save_provider_key",
      provider: "groq",
      key: "secret",
    });

    expect(outcome.mirror).toBeNull();
    expect(outcome).toMatchObject({
      problem: "The setting could not be saved.",
    });
  });
});

describe("clearing a provider key", () => {
  it("returns the refreshed provider status to mirror, and writes no settings", async () => {
    const { writer, calls } = harness();

    const outcome = await writer.change({
      kind: "clear_provider_key",
      provider: "groq",
    });

    expect(calls.clearedKeys).toEqual(["groq"]);
    expect(calls.saves).toEqual([]);
    expect(mirroredStatus(outcome)).toEqual(cleared);
  });

  it("does not mirror the status when the keychain refused, and gives the one write-failure sentence", async () => {
    const { writer } = harness({
      clearProviderKey: async () => {
        throw new Error("the keychain refused");
      },
    });

    const outcome = await writer.change({
      kind: "clear_provider_key",
      provider: "azure",
    });

    expect(outcome.mirror).toBeNull();
    expect(outcome).toMatchObject({
      problem: "The setting could not be saved.",
    });
  });
});

describe("resetting shortcuts and the language pair", () => {
  it("mirrors the shipped settings the shell wrote", async () => {
    const { writer, calls } = harness({
      reset: async () => ({
        settings: shipped,
        bindings: [],
        save_problem: null,
      }),
    });

    const outcome = await writer.change({ kind: "reset" });

    expect(mirroredSettings(outcome)).toEqual(shipped);
    expect(settingsNotice(outcome)).toBe(
      "Shortcuts and language pair put back to the shipped settings.",
    );
    expect(calls.saves).toEqual([]);
  });

  it("does not mirror when the shipped settings could not be written", async () => {
    const { writer } = harness({
      reset: async () => ({
        settings: shipped,
        bindings: [],
        save_problem:
          "The defaults are live, but the settings file could not be written.",
      }),
    });

    const outcome = await writer.change({ kind: "reset" });

    expect(outcome.mirror).toBeNull();
    expect(outcome).toMatchObject({
      problem:
        "The defaults are live, but the settings file could not be written.",
    });
  });

  it("reports the bindings macOS refused, one sentence each", async () => {
    const { writer } = harness({
      reset: async () => ({
        settings: shipped,
        bindings: [
          {
            action: "replace",
            binding: "Ctrl+Alt+R",
            problem: "macOS would not register Ctrl+Alt+R.",
          },
          {
            action: "quick_translate",
            binding: "Ctrl+Alt+D",
            problem: null,
          },
        ],
        save_problem: null,
      }),
    });

    const outcome = await writer.change({ kind: "reset" });

    expect(outcome.refusals).toEqual(["macOS would not register Ctrl+Alt+R."]);
  });

  it("does not mirror when the reset command itself failed", async () => {
    const { writer } = harness({
      reset: async () => {
        throw new Error("the command refused");
      },
    });

    const outcome = await writer.change({ kind: "reset" });

    expect(outcome.mirror).toBeNull();
    expect(outcome).toMatchObject({
      problem: "The shipped settings could not be restored.",
    });
  });
});
