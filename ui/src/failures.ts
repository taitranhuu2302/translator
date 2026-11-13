/**
 * Turning a typed engine failure into words a person can act on.
 *
 * Both surfaces invest in the same wording: the Quick Translate popup and the
 * Translate Window's result pane. Keeping the rules here means a failure reads
 * the same wherever it surfaced, and `popupState.ts` re-exports them so the
 * popup's public shape did not have to move.
 */
import type {
  CaptureError,
  EngineError,
  ProviderError,
  ProviderId,
} from "./ipc";

export type FailureKind = "offline" | "capture" | "provider";

export interface PopupFailure {
  kind: FailureKind;
  title: string;
  detail: string;
}

/** Turn any engine failure into words the result pane can show. Offline is
 * named as offline; every other provider failure stays a provider failure. */
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
    detail: "Try again.",
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
  if (capture === "nothing_to_replace") {
    return {
      kind: "capture",
      title: "There is no captured selection to replace.",
      detail: "Select text in the app you were working in, then press the shortcut again.",
    };
  }
  if (capture === "not_writable") {
    return {
      kind: "capture",
      title: "This app did not accept the replacement.",
      detail: "Your text was left unchanged. Copy the translation instead.",
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
        "Translator needs a network connection. Reconnect, then try again.",
    };
  }
  if (error === "not_configured") {
    return {
      kind: "provider",
      title: "No translation provider is available.",
      detail: "Add a provider key in Settings, or try again later.",
    };
  }
  // Improve has no keyless fallback, so a missing key must name the provider:
  // "No translation provider is available" would leave the user guessing which
  // key to add.
  if (typeof error === "object" && "missing_key" in error) {
    const name = providerLabel(error.missing_key.provider);
    return {
      kind: "provider",
      title: `${name} needs an API key.`,
      detail: `Add a ${name} key under Provider keys in Settings, then try again.`,
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
    const { provider, cause } = error.authentication_failed;
    // A refused resource still has a working key: pointing the user at the key
    // (or at billing) would send them to the wrong fix.
    if (cause === "resource_not_authorized") {
      return {
        kind: "provider",
        title: `${providerLabel(provider)} refused the request for this resource.`,
        detail:
          "The key is fine. Check the Azure resource in Settings: it may be disabled, suspended, or in another region.",
      };
    }
    return {
      kind: "provider",
      title: "The provider rejected its key.",
      detail: `${providerLabel(provider)} needs a new key in Settings.`,
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
    detail: "Try again.",
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
