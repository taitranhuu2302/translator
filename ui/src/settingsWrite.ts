/**
 * The settings write protocol. One module decides how a change becomes the next
 * whole settings object, whether it was saved, and therefore whether the view
 * may mirror it. Views describe the change; they never build a settings object
 * or decide whether to show the result.
 */
import {
  ipc,
  type Language,
  type RebindResult,
  type ResetOutcome,
  type Settings,
  type ShortcutAction,
} from "./ipc";
import { regionChanged, withAzureRegion } from "./providerStatus";
import { withTarget } from "./settingsView";
import { withShortcut } from "./shortcuts";

/** What the view asked to change. */
export type SettingsMutation =
  | { kind: "target"; target: Language }
  | { kind: "rebind"; action: ShortcutAction; binding: string }
  | { kind: "reset" }
  | { kind: "azure_region"; region: string };

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
}

/**
 * What the view may do after a change. `mirror` is the only value a view may
 * put into its shown state; `null` means the shown settings must stay as they
 * are because nothing was saved. `problem` is the one sentence for the failure
 * class, and `notice` the one sentence for a success that has something to say.
 * `refusals` are the per-shortcut sentences a reset reports.
 */
export type ChangeOutcome =
  | { mirror: Settings; notice: string | null; refusals: string[] }
  | { mirror: null; problem: string; refusals: string[] };

export interface SettingsWriter {
  change(mutation: SettingsMutation): Promise<ChangeOutcome>;
}

/** One sentence for a settings file that could not be read, and one for a
 * write that failed. Both Target and the Azure region fail this way, so they
 * share the sentence rather than each naming their own field. */
const READ_FAILED = "The settings could not be read.";
const SAVE_FAILED = "The setting could not be saved.";
const REBIND_FAILED = "The shortcut could not be changed.";
const RESET_FAILED = "The shipped settings could not be restored.";

function refused(problem: string): ChangeOutcome {
  return { mirror: null, problem, refusals: [] };
}

export function createSettingsWriter(
  effects: SettingsWriteEffects,
): SettingsWriter {
  return {
    async change(mutation: SettingsMutation): Promise<ChangeOutcome> {
      switch (mutation.kind) {
        case "target": {
          let current: Settings;
          try {
            current = await effects.read();
          } catch {
            return refused(READ_FAILED);
          }
          const pair = withTarget(current.language_pair, mutation.target);
          if (pair === current.language_pair) {
            return { mirror: current, notice: null, refusals: [] };
          }
          const next = { ...current, language_pair: pair };
          try {
            await effects.save(next);
          } catch {
            return refused(SAVE_FAILED);
          }
          return { mirror: next, notice: "Language pair saved.", refusals: [] };
        }
        case "azure_region": {
          let current: Settings;
          try {
            current = await effects.read();
          } catch {
            return refused(READ_FAILED);
          }
          if (!regionChanged(current, mutation.region)) {
            return { mirror: current, notice: null, refusals: [] };
          }
          const next = withAzureRegion(current, mutation.region);
          try {
            await effects.save(next);
          } catch {
            return refused(SAVE_FAILED);
          }
          return {
            mirror: next,
            notice:
              next.azure_region.length === 0
                ? "Region cleared: a global Azure resource sends no region header."
                : `Region saved: ${next.azure_region}.`,
            refusals: [],
          };
        }
        case "rebind": {
          let current: Settings;
          try {
            current = await effects.read();
          } catch {
            return refused(READ_FAILED);
          }
          let result: RebindResult;
          try {
            result = await effects.rebind(mutation.action, mutation.binding);
          } catch {
            return refused(REBIND_FAILED);
          }
          if (result.problem) {
            // The engine already chose one sentence for this refusal class;
            // the view draws it rather than adding wording.
            return refused(result.problem);
          }
          return {
            mirror: withShortcut(current, mutation.action, result.binding),
            notice: null,
            refusals: [],
          };
        }
        case "reset": {
          // The command reads the current settings itself and writes the whole
          // shipped object, so a UI read would only add a way to fail before
          // the reset is attempted. Its answer is the value to mirror.
          let outcome: ResetOutcome;
          try {
            outcome = await effects.reset();
          } catch {
            return refused(RESET_FAILED);
          }
          const refusals = outcome.bindings
            .filter((status) => status.problem !== null)
            .map((status) => status.problem ?? "");
          if (outcome.save_problem) {
            return { mirror: null, problem: outcome.save_problem, refusals };
          }
          return {
            mirror: outcome.settings,
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
    },
  };
}

/** The writer every settings view uses. The shell's IPC module is the only
 * implementation of the effects; tests inject their own through
 * `createSettingsWriter`. */
export const settingsWriter: SettingsWriter = createSettingsWriter({
  read: () => ipc.getSettings(),
  save: (settings) => ipc.saveSettings(settings),
  rebind: (action, binding) => ipc.rebindShortcut(action, binding),
  reset: () => ipc.resetShortcutsAndLanguagePair(),
});
