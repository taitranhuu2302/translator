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
}

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
} as const;

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
