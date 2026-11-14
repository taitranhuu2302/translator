/**
 * Turning a typed engine failure into the words a surface shows.
 *
 * A notice is the whole of what a surface may say about a failure: one styling
 * kind, one title, one detail, and at most one action. Quick Translate and the
 * Translate Window both draw this model and add nothing of their own, so a
 * failure reads the same wherever it surfaced and a wording change is one edit
 * here.
 */
import { providerLabel } from "./format";
import type { CaptureError, EngineError, ProviderError } from "./ipc";

/** The styling kind. It is derived from the model, never chosen by a surface.
 * `ok` marks a cause that has been resolved, such as a grant that landed while
 * the window stayed open; `error` is for an engine failure that is neither a
 * capture nor a provider failure. */
export type NoticeKind =
  | "offline"
  | "capture"
  | "provider"
  | "error"
  | "ok";

/** The one command a notice may offer. `id` is what a surface wires up; the
 * label is the notice's own words, so no surface names the action itself. */
export interface NoticeAction {
  id: "open_accessibility_settings";
  label: string;
}

/** What a surface draws for a failure, or for the resolved state that replaces
 * one. There is exactly one of these per outcome, and no surface adds, trims,
 * or rewords its text. */
export interface Notice {
  kind: NoticeKind;
  title: string;
  detail: string;
  action?: NoticeAction;
}

/** Turn any engine failure into a notice. */
export function noticeFor(error: EngineError): Notice {
  if ("capture" in error) {
    return noticeForCapture(error.capture);
  }
  if ("provider" in error) {
    return noticeForProvider(error.provider);
  }
  // Speech, history, settings and credentials failures do not reach the two
  // translate surfaces today. They still get one notice rather than nothing,
  // and it is an error rather than a provider's fault.
  return {
    kind: "error",
    title: "The translation did not finish.",
    detail: "Try again.",
  };
}

/**
 * The notice that replaces an Accessibility failure once the grant has landed
 * while the window stayed open. The cause is gone, so it is not a failure; it
 * still comes from the model, so a surface adds no sentence of its own.
 */
export function accessibilityGrantedNotice(shortcut: string): Notice {
  return {
    kind: "ok",
    title: "Accessibility is on.",
    detail: `Select text in any app and press ${shortcut} again.`,
  };
}

/** True for the one notice whose fix can land while a surface stays open. */
export function isAccessibilityNotice(notice: Notice): boolean {
  return notice.action?.id === "open_accessibility_settings";
}

/** Turn a capture failure into a notice. Every capture kind is handled here. */
export function noticeForCapture(capture: CaptureError): Notice {
  if (capture === "no_accessibility") {
    return {
      kind: "capture",
      title: "Translator needs Accessibility to read your selection.",
      detail:
        "Open System Settings → Privacy & Security → Accessibility, enable Translator, then press the shortcut again.",
      action: {
        id: "open_accessibility_settings",
        label: "Open System Settings",
      },
    };
  }
  if (capture === "secure_input") {
    return {
      kind: "capture",
      title: "This is a secure field, so nothing was read.",
      detail:
        "Passwords stay on your Mac. Finish there, then try again elsewhere.",
    };
  }
  if (capture === "nothing_selected") {
    return {
      kind: "capture",
      title: "Select something first.",
      detail: "Select text in any app, then press the shortcut again.",
    };
  }
  if (capture === "nothing_to_replace") {
    return {
      kind: "capture",
      title: "There is no captured selection to replace.",
      detail:
        "Select text in the app you were working in, then press the shortcut again.",
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
  // A wire value this build does not know still reaches the user as a notice.
  return {
    kind: "capture",
    title: "The selection could not be read.",
    detail: "Try again.",
  };
}

function noticeForProvider(error: ProviderError): Notice {
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

