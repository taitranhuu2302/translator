import { describe, expect, it } from "vitest";

import type { ProviderStatus, Settings } from "./ipc";
import { createSettingsWriter, type SettingsWriteEffects } from "./settingsWrite";
import {
  AZURE_ROW,
  GROQ_ROW,
  createProviderKeyRow,
  isConfigured,
  type ProviderKeyRowConfig,
  type ProviderKeyRowEffects,
} from "./providerKey";

const settings: Settings = {
  language_pair: { target: "en", partner: "vi" },
  shortcuts: {
    quick_translate: "Ctrl+Alt+D",
    replace: "Ctrl+Alt+R",
    read_aloud: "Ctrl+Alt+S",
    translate_window: "Ctrl+Alt+E",
  },
  start_at_login: false,
  azure_region: "westeurope",
  onboarding_seen: true,
  auto_check_updates: true,
  save_history: false,
};

function status(overrides: Partial<ProviderStatus> = {}): ProviderStatus {
  return {
    azure_configured: false,
    groq_configured: false,
    azure_skipped: false,
    groq_skipped: false,
    ...overrides,
  };
}

/** The same status with this provider's key stored. */
function withKey(provider: ProviderKeyRowConfig["provider"]): ProviderStatus {
  return status(
    provider === "azure"
      ? { azure_configured: true }
      : { groq_configured: true },
  );
}

interface Calls {
  statusReads: number;
  savedKeys: { provider: string; key: string }[];
  clearedKeys: string[];
  settingsSaves: Settings[];
  settingsSaved: Settings[];
}

/** A row wired to the real settings write, with fake shell effects. The write
 * protocol under it is the production one, so a row test that observes the
 * effects has also observed the one write path. */
function harness(
  config: ProviderKeyRowConfig,
  options: {
    initialStatus?: ProviderStatus;
    readStatus?: () => Promise<ProviderStatus>;
    effects?: Partial<SettingsWriteEffects>;
  } = {},
): { row: ReturnType<typeof createProviderKeyRow>; calls: Calls } {
  const calls: Calls = {
    statusReads: 0,
    savedKeys: [],
    clearedKeys: [],
    settingsSaves: [],
    settingsSaved: [],
  };
  const effects: SettingsWriteEffects = {
    read: async () => settings,
    save: async (next) => {
      calls.settingsSaves.push(next);
    },
    rebind: async (_action, binding) => ({ binding, problem: null }),
    reset: async () => ({ settings, bindings: [], save_problem: null }),
    saveProviderKey: async (provider, key) => {
      calls.savedKeys.push({ provider, key });
      return withKey(config.provider);
    },
    clearProviderKey: async (provider) => {
      calls.clearedKeys.push(provider);
      return status();
    },
    ...options.effects,
  };
  const rowEffects: ProviderKeyRowEffects = {
    readStatus: async () => {
      calls.statusReads += 1;
      return (options.readStatus ?? (async () => options.initialStatus ?? status()))();
    },
    writer: createSettingsWriter(effects),
    settingsSaved: (next) => calls.settingsSaved.push(next),
  };
  return {
    row: createProviderKeyRow(config, rowEffects, {
      region: config.region ? config.region.read(settings) : "",
    }),
    calls,
  };
}

const ROWS: ProviderKeyRowConfig[] = [AZURE_ROW, GROQ_ROW];

describe("loading one provider's key row", () => {
  for (const config of ROWS) {
    it(`reads a ${config.provider} key stored before this change as configured`, async () => {
      const { row, calls } = harness(config, {
        initialStatus: withKey(config.provider),
      });

      await row.load();

      expect(calls.statusReads).toBe(1);
      expect(isConfigured(config.provider, row.state.status)).toBe(true);
      expect(config.statusLine(row.state.status!)).toContain("Key saved");
      expect(config.keyPlaceholder(true)).toContain("Saved");
    });

    it(`reads a cleared ${config.provider} key as not configured`, async () => {
      const { row } = harness(config, { initialStatus: status() });

      await row.load();

      expect(isConfigured(config.provider, row.state.status)).toBe(false);
      expect(config.statusLine(row.state.status!)).toContain("No key");
      expect(config.keyPlaceholder(false)).toContain("Paste");
    });

    it(`reports a ${config.provider} status that could not be read`, async () => {
      const { row } = harness(config, {
        readStatus: async () => {
          throw new Error("the Keychain refused");
        },
      });

      await row.load();

      expect(row.state.status).toBeNull();
      expect(row.state.statusProblem).toBe(
        "The provider status could not be read: the Keychain refused",
      );
    });
  }

  it("keeps the two providers' configured states separate", async () => {
    const azure = harness(AZURE_ROW, {
      initialStatus: status({ azure_configured: true }),
    });
    const groq = harness(GROQ_ROW, {
      initialStatus: status({ azure_configured: true }),
    });

    await azure.row.load();
    await groq.row.load();

    expect(isConfigured("azure", azure.row.state.status)).toBe(true);
    expect(isConfigured("groq", groq.row.state.status)).toBe(false);
  });
});

describe("saving one provider's key", () => {
  for (const config of ROWS) {
    it(`writes a ${config.provider} key through the one settings write, clears the draft, and says it is saved`, async () => {
      const { row, calls } = harness(config, { initialStatus: status() });
      row.setDraft("  secret \n");

      const saving = row.save();
      expect(row.state.busy).toBe(true);
      await saving;

      expect(calls.savedKeys).toEqual([
        { provider: config.provider, key: "secret" },
      ]);
      expect(calls.settingsSaves).toEqual([]);
      expect(row.state.busy).toBe(false);
      expect(row.state.draft).toBe("");
      expect(isConfigured(config.provider, row.state.status)).toBe(true);
      expect(row.state.notice).toBe(config.savedNotice);
    });

    it(`refuses an empty ${config.provider} draft without calling the shell`, async () => {
      const { row, calls } = harness(config);

      row.setDraft("   \n");
      await row.save();

      expect(calls.savedKeys).toEqual([]);
      expect(row.state.busy).toBe(false);
      expect(row.state.draft).toBe("   \n");
      expect(row.state.notice).toBe("Enter a key before saving.");
    });

    it(`does not mirror a failed ${config.provider} save, and keeps the draft`, async () => {
      const { row } = harness(config, {
        initialStatus: withKey(config.provider),
        effects: {
          saveProviderKey: async () => {
            throw new Error("the Keychain refused");
          },
        },
      });
      await row.load();
      row.setDraft("secret");

      await row.save();

      expect(row.state.notice).toBe("The setting could not be saved.");
      expect(row.state.draft).toBe("secret");
      expect(isConfigured(config.provider, row.state.status)).toBe(true);
      expect(row.state.busy).toBe(false);
    });
  }
});

describe("clearing one provider's key", () => {
  for (const config of ROWS) {
    it(`clears a ${config.provider} key through the one settings write and says what happens without it`, async () => {
      const { row, calls } = harness(config, {
        initialStatus: withKey(config.provider),
      });
      await row.load();
      row.setDraft("leftover");

      await row.clear();

      expect(calls.clearedKeys).toEqual([config.provider]);
      expect(row.state.draft).toBe("");
      expect(isConfigured(config.provider, row.state.status)).toBe(false);
      expect(row.state.notice).toBe(config.clearedNotice);
    });

    it(`does not mirror a failed ${config.provider} clear, and gives the one write-failure sentence`, async () => {
      const { row } = harness(config, {
        initialStatus: withKey(config.provider),
        effects: {
          clearProviderKey: async () => {
            throw new Error("the Keychain refused");
          },
        },
      });
      await row.load();

      await row.clear();

      expect(row.state.notice).toBe("The setting could not be saved.");
      expect(isConfigured(config.provider, row.state.status)).toBe(true);
      expect(row.state.busy).toBe(false);
    });
  }
});

describe("a row that is busy", () => {
  for (const config of ROWS) {
    it(`stays busy while a ${config.provider} write is in flight and refuses a second one`, async () => {
      let release!: (next: ProviderStatus) => void;
      const pending = new Promise<ProviderStatus>((resolve) => {
        release = resolve;
      });
      const { row, calls } = harness(config, {
        effects: { saveProviderKey: async () => pending },
      });
      row.setDraft("secret");

      const saving = row.save();
      expect(row.state.busy).toBe(true);
      await row.clear();
      expect(calls.clearedKeys).toEqual([]);

      release(withKey(config.provider));
      await saving;
      expect(row.state.busy).toBe(false);
      expect(row.state.notice).toBe(config.savedNotice);
    });
  }
});

describe("the Azure region", () => {
  it("saves through the same settings write and mirrors the trimmed value", async () => {
    const { row, calls } = harness(AZURE_ROW);
    row.setRegion("  eastus \n");

    await row.saveRegion();

    const expected: Settings = { ...settings, azure_region: "eastus" };
    expect(calls.settingsSaves).toEqual([expected]);
    expect(calls.settingsSaved).toEqual([expected]);
    expect(row.state.region).toBe("eastus");
    expect(row.state.notice).toBe("Region saved: eastus.");
  });

  it("keeps the typed region when the save fails and uses the one sentence", async () => {
    const { row, calls } = harness(AZURE_ROW, {
      effects: {
        save: async () => {
          throw new Error("the settings file refused");
        },
      },
    });
    row.setRegion("eastus");

    await row.saveRegion();

    expect(row.state.region).toBe("eastus");
    expect(row.state.notice).toBe("The setting could not be saved.");
    expect(calls.settingsSaved).toEqual([]);
  });

  it("gives a key-only provider no region field and no region write", async () => {
    const { row, calls } = harness(GROQ_ROW);

    row.setRegion("eastus");
    await row.saveRegion();

    expect(row.state.region).toBe("");
    expect(calls.settingsSaves).toEqual([]);
  });
});

describe("the two row configurations", () => {
  it("are the same module with different words and only Azure carrying a region", () => {
    expect(AZURE_ROW.provider).toBe("azure");
    expect(GROQ_ROW.provider).toBe("groq");
    expect(AZURE_ROW.region).toBeDefined();
    expect(GROQ_ROW.region).toBeUndefined();
  });
});
