/**
 * The preview harness's stand-in for the engine.
 *
 * The settings sheet talks to `ipc` for real when the app runs; in a browser
 * every one of those calls would reject, and the sheet would be reviewed full of
 * errors instead of composed from data. So the harness swaps the commands the
 * settings surface uses for fixed answers. Only `preview/main.tsx` imports this,
 * so the app never sees it.
 */
import {
  ipc,
  type ProviderId,
  type ProviderStatus,
  type Settings,
  type ShortcutAction,
  type ShortcutStatus,
} from "../ipc";

export const MOCK_SETTINGS: Settings = {
  language_pair: { target: "vi", partner: "en" },
  shortcuts: {
    quick_translate: "Ctrl+Alt+D",
    replace: "Ctrl+Alt+R",
    read_aloud: "Ctrl+Alt+S",
    translate_window: "Ctrl+Alt+E",
  },
  start_at_login: true,
  azure_region: "",
  onboarding_seen: true,
};

const STATUS: ProviderStatus = {
  azure_configured: true,
  groq_configured: false,
  azure_skipped: false,
  groq_skipped: false,
};

/** One shortcut macOS refused at launch, so the warning row has something to
 * draw. It is a real failure shape, not invented markup. */
const CONFLICTS: ShortcutStatus[] = [
  {
    action: "read_aloud",
    binding: "Ctrl+Alt+S",
    problem: "Another app is already using this shortcut.",
  },
];

export function mockIpc() {
  Object.assign(ipc, {
    shortcutConflicts: async () => CONFLICTS,
    startAtLoginEnabled: async () => true,
    setStartAtLogin: async (enabled: boolean) => ({
      enabled,
      settings: MOCK_SETTINGS,
      problem: null,
    }),
    providerStatus: async () => STATUS,
    getSettings: async () => MOCK_SETTINGS,
    saveSettings: async () => undefined,
    saveProviderKey: async (_provider: ProviderId, _key: string) => STATUS,
    clearProviderKey: async (_provider: ProviderId) => STATUS,
    rebindShortcut: async (_action: ShortcutAction, binding: string) => ({
      binding,
      problem: null,
    }),
    resetShortcutsAndLanguagePair: async () => ({
      settings: MOCK_SETTINGS,
      bindings: [],
      save_problem: null,
    }),
  });
}
