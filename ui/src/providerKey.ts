/**
 * One provider's key row: whether the key is stored, the write-only draft,
 * save and clear through the one settings write, and the sentence that follows.
 *
 * Azure and Groq are two configurations of this module. Only the words and the
 * optional region differ; the save path, the clear path, the busy rule and the
 * mirror rule are the same code. The component that draws a row holds no rules
 * of its own.
 */
import type { ProviderId, ProviderStatus, Settings } from "./ipc";
import {
  azureStatusLine,
  groqStatusLine,
  regionChanged,
} from "./providerStatus";
import type { SettingsWriter } from "./settingsWrite";

/** The providers the Settings surface stores a key for. MyMemory has none. */
export type ProviderKeyProvider = Extract<ProviderId, "azure" | "groq">;

/** Everything that differs between two provider rows. A new provider is one
 * more value of this type, not a copied handler and a copied block of markup. */
export interface ProviderKeyRowConfig {
  provider: ProviderKeyProvider;
  title: string;
  /** The sentence under the title for the status the shell reported. */
  statusLine(status: ProviderStatus): string;
  /** What the write-only key field says before anything is typed. */
  keyPlaceholder(configured: boolean): string;
  /** The sentence a saved key leaves behind. */
  savedNotice: string;
  /** The sentence a cleared key leaves behind. */
  clearedNotice: string;
  /** Azure's optional resource region; a key-only provider has none. */
  region?: {
    read(settings: Settings): string;
    changed(settings: Settings, region: string): boolean;
  };
}

export const AZURE_ROW: ProviderKeyRowConfig = {
  provider: "azure",
  title: "Azure Translator",
  statusLine: azureStatusLine,
  keyPlaceholder: (configured) =>
    configured
      ? "Saved — enter a new key to replace it"
      : "Paste an Azure Translator key",
  savedNotice: "Azure key saved in the Keychain.",
  clearedNotice: "Azure key cleared. MyMemory translates without one.",
  region: {
    read: (settings) => settings.azure_region,
    changed: regionChanged,
  },
};

export const GROQ_ROW: ProviderKeyRowConfig = {
  provider: "groq",
  title: "Groq (Improve)",
  statusLine: groqStatusLine,
  keyPlaceholder: (configured) =>
    configured
      ? "Saved — enter a new key to replace it"
      : "Paste a Groq API key",
  savedNotice: "Groq key saved in the Keychain.",
  clearedNotice: "Groq key cleared. Improve will name the key it needs.",
};

/** What one row shows. The page holds none of this: each row owns its draft,
 * its busy state and its notice. */
export interface ProviderKeyRowState {
  /** The whole status the shell last answered; null until the read lands. */
  status: ProviderStatus | null;
  /** The sentence when the status could not be read. */
  statusProblem: string | null;
  /** The write-only key field. The key is never read back into it. */
  draft: string;
  /** The Azure region field; empty for a provider with none. */
  region: string;
  busy: boolean;
  /** The sentence that follows the last save or clear, if any. */
  notice: string | null;
}

/** The one sentence for a save the row itself refuses. */
const EMPTY_DRAFT_NOTICE = "Enter a key before saving.";

/** The shell operations a row uses. The writer is ticket 05's one settings
 * write; `settingsSaved` carries a region save's mirrored settings back to the
 * page, which owns the shown `Settings`. */
export interface ProviderKeyRowEffects {
  readStatus(): Promise<ProviderStatus>;
  writer: SettingsWriter;
  settingsSaved(settings: Settings): void;
}

/** The row as the component sees it: a subscribable state plus the four
 * actions, all of which answer through the one writer. */
export interface ProviderKeyRowModel {
  readonly state: ProviderKeyRowState;
  subscribe(listener: () => void): () => void;
  load(): Promise<void>;
  setDraft(value: string): void;
  setRegion(value: string): void;
  save(): Promise<void>;
  clear(): Promise<void>;
  saveRegion(): Promise<void>;
}

/** True when this provider's key is stored. The one place the status's two
 * flags become a provider's answer. */
export function isConfigured(
  provider: ProviderKeyProvider,
  status: ProviderStatus | null,
): boolean {
  if (!status) return false;
  return provider === "azure"
    ? status.azure_configured
    : status.groq_configured;
}

export function createProviderKeyRow(
  config: ProviderKeyRowConfig,
  effects: ProviderKeyRowEffects,
  initial: { region?: string } = {},
): ProviderKeyRowModel {
  let state: ProviderKeyRowState = {
    status: null,
    statusProblem: null,
    draft: "",
    region: initial.region ?? "",
    busy: false,
    notice: null,
  };
  const listeners = new Set<() => void>();

  function publish(next: ProviderKeyRowState): void {
    state = next;
    for (const listener of listeners) listener();
  }

  return {
    get state() {
      return state;
    },

    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },

    async load() {
      try {
        const status = await effects.readStatus();
        publish({ ...state, status, statusProblem: null });
      } catch (reason) {
        publish({ ...state, statusProblem: describe(reason) });
      }
    },

    setDraft(value) {
      publish({ ...state, draft: value });
    },

    setRegion(value) {
      if (!config.region) return;
      publish({ ...state, region: value });
    },

    // The save and clear paths are separate user actions, but both go through
    // the one settings write and both use the same busy guard, the same mirror
    // rule and the same failure sentence.
    async save() {
      if (state.busy) return;
      const key = state.draft.trim();
      if (key.length === 0) {
        publish({ ...state, notice: EMPTY_DRAFT_NOTICE });
        return;
      }
      publish({ ...state, busy: true, notice: null });
      const outcome = await effects.writer.change({
        kind: "save_provider_key",
        provider: config.provider,
        key,
      });
      if (outcome.mirror === null) {
        publish({ ...state, busy: false, notice: outcome.problem });
        return;
      }
      publish({
        ...state,
        busy: false,
        status: outcome.mirror.status,
        draft: "",
        notice: config.savedNotice,
      });
    },

    async clear() {
      if (state.busy) return;
      publish({ ...state, busy: true, notice: null });
      const outcome = await effects.writer.change({
        kind: "clear_provider_key",
        provider: config.provider,
      });
      if (outcome.mirror === null) {
        publish({ ...state, busy: false, notice: outcome.problem });
        return;
      }
      publish({
        ...state,
        busy: false,
        status: outcome.mirror.status,
        draft: "",
        notice: config.clearedNotice,
      });
    },

    async saveRegion() {
      if (!config.region || state.busy) return;
      publish({ ...state, busy: true, notice: null });
      const outcome = await effects.writer.change({
        kind: "azure_region",
        region: state.region,
      });
      if (outcome.mirror === null) {
        publish({ ...state, busy: false, notice: outcome.problem });
        return;
      }
      effects.settingsSaved(outcome.mirror.settings);
      publish({
        ...state,
        busy: false,
        region: outcome.mirror.settings.azure_region,
        notice: outcome.notice,
      });
    },
  };
}

/** Errors arrive as serde-tagged values; show the message, never a raw object. */
function describe(reason: unknown): string {
  if (typeof reason === "string") return reason;
  if (reason instanceof Error) return reason.message;
  return JSON.stringify(reason);
}
