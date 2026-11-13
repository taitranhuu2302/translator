/**
 * The frontend's only seam to the shell.
 *
 * Every command and event is wrapped here with hand-written types that mirror the
 * Rust engine's serde shapes. Components import this module; no component imports
 * `@tauri-apps/api` directly (scripts/test.sh enforces that).
 */
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// --- Model -----------------------------------------------------------------

export type Language = string;

export type Mode = "translate" | "improve";

export type ProviderId = "azure" | "my_memory" | "groq";

export interface LanguagePair {
  target: Language;
  partner: Language;
}

export interface ShortcutBindings {
  quick_translate: string;
  replace: string;
  read_aloud: string;
  translate_window: string;
}

export interface Settings {
  language_pair: LanguagePair;
  shortcuts: ShortcutBindings;
  start_at_login: boolean;
  onboarding_seen: boolean;
}

/** The text the user selected when a shortcut fired. */
export interface Selection {
  text: string;
}

/**
 * What the Translate Window starts with. A missing Selection is an empty
 * `selection`, never a failure; a missing permission is a typed failure.
 */
export type Prefill =
  | { selection: Selection }
  | { failure: EngineError };

export interface Translation {
  text: string;
  provider: ProviderId;
  detected_source: Language | null;
  target: Language;
}

export interface Improvement {
  corrected: string;
  better: string;
}

export type TextOutcome =
  | { translation: Translation }
  | { improvement: Improvement };

export interface QuickTranslation {
  source_text: string;
  translation: Translation;
}

export interface Replacement {
  source_text: string;
  translation: Translation;
}

export interface ReadAloud {
  spoken_text: string;
}

export interface HistoryEntry {
  source: string;
  result: string;
  provider: ProviderId;
  mode: Mode;
  created_at: number;
}

export interface ProviderStatus {
  azure_configured: boolean;
  groq_configured: boolean;
}

// --- Errors ----------------------------------------------------------------

export type CaptureError =
  | "no_accessibility"
  | "secure_input"
  | "nothing_selected"
  | "not_writable"
  | { unavailable: { reason: string } };

export type ProviderError =
  | "not_configured"
  | "offline"
  | { quota_exceeded: { provider: ProviderId } }
  | { authentication_failed: { provider: ProviderId } }
  | { network: { provider: ProviderId; message: string } }
  | { malformed_response: { provider: ProviderId; message: string } }
  | { unavailable: { reason: string } };

export type SpeechError = { unavailable: { reason: string } };

export type HistoryError =
  | { unavailable: { reason: string } }
  | { storage: { message: string } };

export type SettingsError =
  | { unavailable: { reason: string } }
  | { storage: { message: string } };

export type CredentialError =
  | { unavailable: { reason: string } }
  | { storage: { message: string } };

/** Every engine outcome is a typed success or one of these. */
export type EngineError =
  | { capture: CaptureError }
  | { provider: ProviderError }
  | { speech: SpeechError }
  | { history: HistoryError }
  | { settings: SettingsError }
  | { credentials: CredentialError };

// --- Commands --------------------------------------------------------------

function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args);
}

export const ipc = {
  quickTranslate: (): Promise<QuickTranslation> => call("quick_translate"),
  replaceSelection: (): Promise<Replacement> => call("replace_selection"),
  readAloud: (): Promise<ReadAloud> => call("read_aloud"),
  translateText: (text: string, mode: Mode): Promise<TextOutcome> =>
    call("translate_text", { text, mode }),
  providerStatus: (): Promise<ProviderStatus> => call("provider_status"),
  getSettings: (): Promise<Settings> => call("get_settings"),
  saveSettings: (settings: Settings): Promise<void> =>
    call("save_settings", { settings }),
  searchHistory: (query: string, limit: number): Promise<HistoryEntry[]> =>
    call("search_history", { query, limit }),

  /** The Selection captured when the window was opened, or null if none is
   * waiting. Capture happens before the window is shown, so this is the only
   * way the input pane learns what was selected. */
  takeSelectionPrefill: (): Promise<Prefill | null> =>
    call("take_selection_prefill"),

  accessibilityGranted: (): Promise<boolean> => call("accessibility_granted"),

  /** Ask macOS to prompt for Accessibility; resolves to the state after asking. */
  requestAccessibility: (): Promise<boolean> => call("request_accessibility"),

  /** Open System Settings at Privacy & Security → Accessibility. */
  openAccessibilitySettings: (): Promise<void> => call("open_accessibility_settings"),

  completeOnboarding: (): Promise<Settings> => call("complete_onboarding"),
};

// --- Events ----------------------------------------------------------------

/**
 * Stream shapes. Translation and Improve share one event shape so the views have
 * one code path; the emitters land with the streaming tickets.
 */
export interface TranslationChunk {
  request_id: string;
  text: string;
}

export interface TranslationDone {
  request_id: string;
  outcome: TextOutcome;
}

export interface TranslationError {
  request_id: string;
  error: EngineError;
}

export interface ProviderStatusEvent {
  status: ProviderStatus;
}

export const EVENTS = {
  translationChunk: "translation-chunk",
  translationDone: "translation-done",
  translationError: "translation-error",
  providerStatus: "provider-status",
  selectionPrefill: "selection-prefill",
} as const;

/** Emitted after the window is opened from a Selection. The payload is empty:
 * call `takeSelectionPrefill` to claim what was captured. */
export function onSelectionPrefill(handler: () => void): Promise<UnlistenFn> {
  return listen(EVENTS.selectionPrefill, () => handler());
}

export function onTranslationChunk(
  handler: (payload: TranslationChunk) => void,
): Promise<UnlistenFn> {
  return listen<TranslationChunk>(EVENTS.translationChunk, (event) =>
    handler(event.payload),
  );
}

export function onTranslationDone(
  handler: (payload: TranslationDone) => void,
): Promise<UnlistenFn> {
  return listen<TranslationDone>(EVENTS.translationDone, (event) =>
    handler(event.payload),
  );
}

export function onTranslationError(
  handler: (payload: TranslationError) => void,
): Promise<UnlistenFn> {
  return listen<TranslationError>(EVENTS.translationError, (event) =>
    handler(event.payload),
  );
}

export function onProviderStatus(
  handler: (payload: ProviderStatusEvent) => void,
): Promise<UnlistenFn> {
  return listen<ProviderStatusEvent>(EVENTS.providerStatus, (event) =>
    handler(event.payload),
  );
}
