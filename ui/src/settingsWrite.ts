/**
 * The settings write protocol. One module decides how a change becomes the next
 * whole settings object, whether it was saved, and therefore whether the view
 * may mirror it. Views describe the change; they never build a settings object
 * or decide whether to show the result.
 */
import {
  ipc,
  type Language,
  type ProviderId,
  type ProviderStatus,
  type RebindResult,
  type ResetOutcome,
  type Settings,
  type ShortcutAction,
} from "./ipc";
import { regionChanged, withAzureRegion } from "./providerStatus";
import { withTarget } from "./settingsView";
import { withShortcut } from "./shortcuts";

/** What the view asked to change. Every write the Settings surface makes is one
 * of these; a provider key save and a clear each have their own mutation
 * because they have their own user action and their own success sentence. */
export type SettingsMutation =
  | { kind: "target"; target: Language }
  | { kind: "rebind"; action: ShortcutAction; binding: string }
  | { kind: "reset" }
  | { kind: "azure_region"; region: string }
  | { kind: "auto_check_updates"; enabled: boolean }
  | { kind: "save_provider_key"; provider: ProviderId; key: string }
  | { kind: "clear_provider_key"; provider: ProviderId };

/** The changes that answer with a settings object to mirror. */
export type PlainSettingsMutation = Exclude<
  SettingsMutation,
  { kind: "reset" } | { kind: "save_provider_key" } | { kind: "clear_provider_key" }
>;

/** The changes that answer with a refreshed provider status to mirror. */
export type ProviderKeyMutation = Extract<
  SettingsMutation,
  { kind: "save_provider_key" } | { kind: "clear_provider_key" }
>;

/** A reset additionally answers with the bindings macOS refused. */
export type ResetMutation = Extract<SettingsMutation, { kind: "reset" }>;

/** What a change may put into a view's shown state. The tag says which state
 * channel the value belongs to, so a settings view and a key row each get the
 * value they can draw without deciding between two mirrors themselves. */
export type SettingsMirror = { kind: "settings"; settings: Settings };
export type ProviderStatusMirror = {
  kind: "provider_status";
  status: ProviderStatus;
};
export type Mirror = SettingsMirror | ProviderStatusMirror;

/**
 * What the view may do after a change. `mirror` is the only value a view may
 * put into its shown state; `null` means the shown state must stay as it is
 * because nothing was saved. `problem` is the one sentence for the failure
 * class, and `notice` the one sentence for a success that has something to say.
 */
export type SettingsChange =
  | { mirror: SettingsMirror; notice: string | null }
  | { mirror: null; problem: string };

export type ProviderKeyChange =
  | { mirror: ProviderStatusMirror; notice: string | null }
  | { mirror: null; problem: string };

/** A reset is the only change with per-shortcut refusals, so only its outcome
 * carries them. */
export type ResetChange =
  | { mirror: SettingsMirror; notice: string | null; refusals: string[] }
  | { mirror: null; problem: string; refusals: string[] };

export type ChangeOutcome = SettingsChange | ProviderKeyChange | ResetChange;

/** The shell operations the protocol uses. Injected so the rules run without a
 * window, a Keychain, or a running app. */
export interface SettingsWriteEffects {
  /** The settings the engine has actually saved right now. */
  read(): Promise<Settings>;
  /** Write a complete settings object. */
  save(settings: Settings): Promise<void>;
  /** Apply and save one binding; the shell answers what is live and stored. */
  rebind(action: ShortcutAction, binding: string): Promise<RebindResult>;
  /** Restore the shipped settings, answering what was written. */
  reset(): Promise<ResetOutcome>;
  /** Store one key in the Keychain and answer the refreshed status. */
  saveProviderKey(provider: ProviderId, key: string): Promise<ProviderStatus>;
  /** Forget one key and answer the refreshed status. */
  clearProviderKey(provider: ProviderId): Promise<ProviderStatus>;
}

/**
 * The one way any view writes. The overloads narrow each mutation to the
 * mirror its view can draw: a Target or a region answers settings, a key save
 * or clear answers provider status, and only a reset answers refusals. There is
 * still one implementation and one set of failure sentences behind every call.
 */
export interface SettingsWriter {
  change(mutation: ResetMutation): Promise<ResetChange>;
  change(mutation: ProviderKeyMutation): Promise<ProviderKeyChange>;
  change(mutation: PlainSettingsMutation): Promise<SettingsChange>;
}

/** The one sentence for a settings file that could not be read. It is exported
 * because the surfaces that load settings directly report the same failure and
 * must not word it a second way. */
export const SETTINGS_READ_FAILED = "The settings could not be read.";

/** One sentence for a write that failed. Every write shares it rather than each
 * naming its own field, so the app never explains one failure two ways. */
const SAVE_FAILED = "The setting could not be saved.";
const REBIND_FAILED = "The shortcut could not be changed.";
const RESET_FAILED = "The shipped settings could not be restored.";

function failed(problem: string): { mirror: null; problem: string } {
  return { mirror: null, problem };
}

export function createSettingsWriter(
  effects: SettingsWriteEffects,
): SettingsWriter {
  async function change(mutation: ResetMutation): Promise<ResetChange>;
  async function change(mutation: ProviderKeyMutation): Promise<ProviderKeyChange>;
  async function change(mutation: PlainSettingsMutation): Promise<SettingsChange>;
  async function change(mutation: SettingsMutation): Promise<ChangeOutcome> {
    switch (mutation.kind) {
      case "target": {
        let current: Settings;
        try {
          current = await effects.read();
        } catch {
          return failed(SETTINGS_READ_FAILED);
        }
        const pair = withTarget(current.language_pair, mutation.target);
        if (pair === current.language_pair) {
          return {
            mirror: { kind: "settings", settings: current },
            notice: null,
          };
        }
        const next = { ...current, language_pair: pair };
        try {
          await effects.save(next);
        } catch {
          return failed(SAVE_FAILED);
        }
        return {
          mirror: { kind: "settings", settings: next },
          notice: "Language pair saved.",
        };
      }
      case "azure_region": {
        let current: Settings;
        try {
          current = await effects.read();
        } catch {
          return failed(SETTINGS_READ_FAILED);
        }
        if (!regionChanged(current, mutation.region)) {
          return {
            mirror: { kind: "settings", settings: current },
            notice: null,
          };
        }
        const next = withAzureRegion(current, mutation.region);
        try {
          await effects.save(next);
        } catch {
          return failed(SAVE_FAILED);
        }
        return {
          mirror: { kind: "settings", settings: next },
          notice:
            next.azure_region.length === 0
              ? "Region cleared: a global Azure resource sends no region header."
              : `Region saved: ${next.azure_region}.`,
        };
      }
      case "auto_check_updates": {
        let current: Settings;
        try {
          current = await effects.read();
        } catch {
          return failed(SETTINGS_READ_FAILED);
        }
        if (current.auto_check_updates === mutation.enabled) {
          return {
            mirror: { kind: "settings", settings: current },
            notice: null,
          };
        }
        // The shell wakes its update schedule when this field changes as a
        // side effect of the save, so the switch takes effect at once.
        const next = { ...current, auto_check_updates: mutation.enabled };
        try {
          await effects.save(next);
        } catch {
          return failed(SAVE_FAILED);
        }
        return {
          mirror: { kind: "settings", settings: next },
          notice: mutation.enabled
            ? "Automatic update checks turned on."
            : "Automatic update checks turned off.",
        };
      }
      case "rebind": {
        let current: Settings;
        try {
          current = await effects.read();
        } catch {
          return failed(SETTINGS_READ_FAILED);
        }
        let result: RebindResult;
        try {
          result = await effects.rebind(mutation.action, mutation.binding);
        } catch {
          return failed(REBIND_FAILED);
        }
        if (result.problem) {
          // The engine already chose one sentence for this refusal class;
          // the view draws it rather than adding wording.
          return failed(result.problem);
        }
        return {
          mirror: {
            kind: "settings",
            settings: withShortcut(current, mutation.action, result.binding),
          },
          notice: null,
        };
      }
      case "save_provider_key": {
        let status: ProviderStatus;
        try {
          status = await effects.saveProviderKey(
            mutation.provider,
            mutation.key,
          );
        } catch {
          return failed(SAVE_FAILED);
        }
        return { mirror: { kind: "provider_status", status }, notice: null };
      }
      case "clear_provider_key": {
        let status: ProviderStatus;
        try {
          status = await effects.clearProviderKey(mutation.provider);
        } catch {
          return failed(SAVE_FAILED);
        }
        return { mirror: { kind: "provider_status", status }, notice: null };
      }
      case "reset": {
        // The command reads the current settings itself and writes the whole
        // shipped object, so a UI read would only add a way to fail before
        // the reset is attempted. Its answer is the value to mirror.
        let outcome: ResetOutcome;
        try {
          outcome = await effects.reset();
        } catch {
          return { ...failed(RESET_FAILED), refusals: [] };
        }
        const refusals = outcome.bindings
          .filter((status) => status.problem !== null)
          .map((status) => status.problem ?? "");
        if (outcome.save_problem) {
          return { ...failed(outcome.save_problem), refusals };
        }
        return {
          mirror: { kind: "settings", settings: outcome.settings },
          notice:
            "Shortcuts and language pair put back to the shipped settings.",
          refusals,
        };
      }
      default: {
        const unknown: never = mutation;
        throw new Error(`unknown settings mutation: ${String(unknown)}`);
      }
    }
  }

  return { change };
}

/** The writer every settings view uses. The shell's IPC module is the only
 * implementation of the effects; tests inject their own through
 * `createSettingsWriter`. */
export const settingsWriter: SettingsWriter = createSettingsWriter({
  read: () => ipc.getSettings(),
  save: (settings) => ipc.saveSettings(settings),
  rebind: (action, binding) => ipc.rebindShortcut(action, binding),
  reset: () => ipc.resetShortcutsAndLanguagePair(),
  saveProviderKey: (provider, key) => ipc.saveProviderKey(provider, key),
  clearProviderKey: (provider) => ipc.clearProviderKey(provider),
});
