/**
 * The Quick Translate popup as a pure state machine.
 *
 * The panel is shown before any network answer exists, so the popup always has
 * a state to draw: opening, translating, translated, nothing selected, a typed
 * failure, or dismissed. Keeping the transitions here means the rules are tested
 * without a window, a provider or a running app.
 */
import type {
  CaptureError,
  EngineError,
  PopupPrefill,
  ProviderError,
  ProviderId,
  Translation,
} from "./ipc";

export type FailureKind = "offline" | "capture" | "provider";

export interface PopupFailure {
  kind: FailureKind;
  title: string;
  detail: string;
}

export type PopupState =
  | { phase: "opening" }
  | { phase: "translating"; source: string }
  | { phase: "translated"; source: string; translation: Translation }
  | { phase: "nothing_selected" }
  | ({ phase: "failed" } & PopupFailure)
  | { phase: "dismissed" };

export type PopupEvent =
  | { type: "prefill"; prefill: PopupPrefill }
  | { type: "translated"; translation: Translation }
  | { type: "failed"; error: EngineError }
  | { type: "dismissed" };

export const OPENING: PopupState = { phase: "opening" };

export function reduce(state: PopupState, event: PopupEvent): PopupState {
  switch (event.type) {
    // The same panel is reused, so a prefill always starts a fresh session —
    // even after the previous answer was dismissed.
    case "prefill": {
      const prefill = event.prefill;
      if (prefill === "nothing_selected") {
        return { phase: "nothing_selected" };
      }
      if ("failure" in prefill) {
        return failedWith(formatCaptureFailure(prefill.failure));
      }
      const source = prefill.selection.text;
      if (source.trim().length === 0) {
        return { phase: "nothing_selected" };
      }
      return { phase: "translating", source };
    }

    case "dismissed":
      return { phase: "dismissed" };

    case "translated":
      // Once dismissed, a late answer must not bring the popup back.
      if (state.phase !== "translating") {
        return state;
      }
      return { phase: "translated", source: state.source, translation: event.translation };

    case "failed":
      if (state.phase !== "translating") {
        return state;
      }
      return failedWith(formatFailure(event.error));
  }
}

function failedWith(failure: PopupFailure): PopupState {
  return { phase: "failed", ...failure };
}

/** Turn any engine failure into words the popup can show. Offline is named as
 * offline; every other provider failure stays a provider failure. */
export function formatFailure(error: EngineError): PopupFailure {
  if ("provider" in error) {
    return formatProviderFailure(error.provider);
  }
  if ("capture" in error) {
    return formatCaptureFailure(error.capture);
  }
  return {
    kind: "provider",
    title: "The translation did not finish.",
    detail: "Try the shortcut again.",
  };
}

export function formatCaptureFailure(capture: CaptureError): PopupFailure {
  if (capture === "no_accessibility") {
    return {
      kind: "capture",
      title: "Translator needs Accessibility to read your selection.",
      detail:
        "Open System Settings → Privacy & Security → Accessibility, enable Translator, then press the shortcut again.",
    };
  }
  if (capture === "secure_input") {
    return {
      kind: "capture",
      title: "This is a secure field, so nothing was read.",
      detail: "Passwords stay on your Mac. Finish there, then try again elsewhere.",
    };
  }
  if (capture === "nothing_selected") {
    return {
      kind: "capture",
      title: "Select something first.",
      detail: "Select text in any app, then press the Quick Translate shortcut.",
    };
  }
  if (typeof capture === "object" && "unavailable" in capture) {
    return {
      kind: "capture",
      title: "The selection could not be read.",
      detail: capture.unavailable.reason,
    };
  }
  return {
    kind: "capture",
    title: "The selection could not be read.",
    detail: "Try again, or use the Translate Window.",
  };
}

function formatProviderFailure(error: ProviderError): PopupFailure {
  if (error === "offline") {
    return {
      kind: "offline",
      title: "You appear to be offline.",
      detail:
        "Quick Translate needs a network connection. Reconnect and press the shortcut again.",
    };
  }
  if (error === "not_configured") {
    return {
      kind: "provider",
      title: "No translation provider is available.",
      detail: "Add a provider key in Settings, or try again later.",
    };
  }
  if ("quota_exceeded" in error) {
    return {
      kind: "provider",
      title: "The free translation quota is used up.",
      detail: `${providerLabel(error.quota_exceeded.provider)} will translate again when the quota resets.`,
    };
  }
  if ("authentication_failed" in error) {
    return {
      kind: "provider",
      title: "The provider rejected its key.",
      detail: `${providerLabel(error.authentication_failed.provider)} needs a new key in Settings.`,
    };
  }
  if ("network" in error) {
    return {
      kind: "provider",
      title: "The translation service could not be reached.",
      detail: `${providerLabel(error.network.provider)}: ${error.network.message}`,
    };
  }
  if ("malformed_response" in error) {
    return {
      kind: "provider",
      title: "The translation service sent something unexpected.",
      detail: `${providerLabel(error.malformed_response.provider)}: ${error.malformed_response.message}`,
    };
  }
  if ("unavailable" in error) {
    return {
      kind: "provider",
      title: "Translation is unavailable right now.",
      detail: error.unavailable.reason,
    };
  }
  return {
    kind: "provider",
    title: "The translation did not finish.",
    detail: "Try the shortcut again.",
  };
}

const PROVIDER_NAMES: Record<ProviderId, string> = {
  azure: "Azure",
  my_memory: "MyMemory",
  groq: "Groq",
};

export function providerLabel(provider: ProviderId): string {
  return PROVIDER_NAMES[provider];
}
