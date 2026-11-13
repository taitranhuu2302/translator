import { useEffect, useState } from "react";

import { ipc, type ProviderStatus, type Settings } from "./ipc";
import {
  azureStatusLine,
  groqStatusLine,
  myMemoryStatusLine,
  regionChanged,
  withAzureRegion,
} from "./providerStatus";

/**
 * The provider rows of the Settings surface: whether Azure and Groq have a key
 * (and whether this session skipped them), a place to enter or clear each key,
 * the optional Azure region, and MyMemory — the keyless fallback that is always
 * there. Saving or clearing a key returns the refreshed status, so a new key
 * stops showing as skipped immediately.
 */
export function ProviderSettings({
  settings,
  onSettingsSaved,
}: {
  settings: Settings;
  onSettingsSaved: (settings: Settings) => void;
}) {
  const [status, setStatus] = useState<ProviderStatus | null>(null);
  const [statusError, setStatusError] = useState<string | null>(null);
  const [keyDraft, setKeyDraft] = useState("");
  const [groqDraft, setGroqDraft] = useState("");
  const [regionDraft, setRegionDraft] = useState(settings.azure_region);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [groqNotice, setGroqNotice] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    ipc
      .providerStatus()
      .then((loaded) => {
        if (!cancelled) setStatus(loaded);
      })
      .catch((reason: unknown) => {
        if (!cancelled) setStatusError(JSON.stringify(reason));
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // Settings can be reloaded elsewhere (onboarding, a future Settings view).
  useEffect(() => {
    setRegionDraft(settings.azure_region);
  }, [settings.azure_region]);

  const saveKey = async () => {
    setBusy(true);
    setNotice(null);
    try {
      setStatus(await ipc.saveProviderKey("azure", keyDraft));
      // The field is a write-only draft: the key itself is never read back.
      setKeyDraft("");
      setNotice("Azure key saved in the Keychain.");
    } catch (reason) {
      setNotice(`The key could not be saved: ${describe(reason)}`);
    } finally {
      setBusy(false);
    }
  };

  const clearKey = async () => {
    setBusy(true);
    setNotice(null);
    try {
      setStatus(await ipc.clearProviderKey("azure"));
      setKeyDraft("");
      setNotice("Azure key cleared. MyMemory translates without one.");
    } catch (reason) {
      setNotice(`The key could not be cleared: ${describe(reason)}`);
    } finally {
      setBusy(false);
    }
  };

  const saveGroqKey = async () => {
    setBusy(true);
    setGroqNotice(null);
    try {
      setStatus(await ipc.saveProviderKey("groq", groqDraft));
      // Write-only draft: the key is never read back into the field.
      setGroqDraft("");
      setGroqNotice("Groq key saved in the Keychain.");
    } catch (reason) {
      setGroqNotice(`The key could not be saved: ${describe(reason)}`);
    } finally {
      setBusy(false);
    }
  };

  const clearGroqKey = async () => {
    setBusy(true);
    setGroqNotice(null);
    try {
      setStatus(await ipc.clearProviderKey("groq"));
      setGroqDraft("");
      setGroqNotice("Groq key cleared. Improve will name the key it needs.");
    } catch (reason) {
      setGroqNotice(`The key could not be cleared: ${describe(reason)}`);
    } finally {
      setBusy(false);
    }
  };

  const saveRegion = async () => {
    const next = withAzureRegion(settings, regionDraft);
    setBusy(true);
    setNotice(null);
    try {
      await ipc.saveSettings(next);
      onSettingsSaved(next);
      setRegionDraft(next.azure_region);
      setNotice(
        next.azure_region.length === 0
          ? "Region cleared: a global Azure resource sends no region header."
          : `Region saved: ${next.azure_region}.`,
      );
    } catch (reason) {
      setNotice(`The region could not be saved: ${describe(reason)}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="provider-settings">
      <div className="provider-head">
        <h2>Azure Translator</h2>
        <span className={status?.azure_configured ? "badge ok" : "badge"}>
          {status?.azure_configured ? "Key saved" : "No key"}
        </span>
      </div>

      <p className="muted small">
        {statusError
          ? `The provider status could not be read: ${statusError}`
          : status
            ? azureStatusLine(status)
            : "Checking the Keychain…"}
      </p>

      <label className="field">
        <span>API key</span>
        <input
          type="password"
          value={keyDraft}
          onChange={(event) => setKeyDraft(event.target.value)}
          placeholder={
            status?.azure_configured
              ? "Saved — enter a new key to replace it"
              : "Paste an Azure Translator key"
          }
          autoComplete="off"
          spellCheck={false}
        />
      </label>

      <label className="field">
        <span>Region (optional)</span>
        <input
          value={regionDraft}
          onChange={(event) => setRegionDraft(event.target.value)}
          placeholder="e.g. westeurope — leave empty for a global resource"
          autoComplete="off"
          spellCheck={false}
        />
      </label>

      <div className="actions">
        <button
          className="primary"
          onClick={saveKey}
          disabled={busy || keyDraft.trim().length === 0}
        >
          Save key
        </button>
        {status?.azure_configured ? (
          <button onClick={clearKey} disabled={busy}>
            Clear key
          </button>
        ) : null}
        <button
          onClick={saveRegion}
          disabled={busy || !regionChanged(settings, regionDraft)}
        >
          Save region
        </button>
      </div>

      {notice ? <p className="muted small">{notice}</p> : null}

      <hr className="provider-divider" />

      <div className="provider-head">
        <h2>Groq (Improve)</h2>
        <span className={status?.groq_configured ? "badge ok" : "badge"}>
          {status?.groq_configured ? "Key saved" : "No key"}
        </span>
      </div>

      <p className="muted small">
        {statusError
          ? `The provider status could not be read: ${statusError}`
          : status
            ? groqStatusLine(status)
            : "Checking the Keychain…"}
      </p>

      <label className="field">
        <span>API key</span>
        <input
          type="password"
          value={groqDraft}
          onChange={(event) => setGroqDraft(event.target.value)}
          placeholder={
            status?.groq_configured
              ? "Saved — enter a new key to replace it"
              : "Paste a Groq API key"
          }
          autoComplete="off"
          spellCheck={false}
        />
      </label>

      <div className="actions">
        <button
          className="primary"
          onClick={saveGroqKey}
          disabled={busy || groqDraft.trim().length === 0}
        >
          Save key
        </button>
        {status?.groq_configured ? (
          <button onClick={clearGroqKey} disabled={busy}>
            Clear key
          </button>
        ) : null}
      </div>

      {groqNotice ? <p className="muted small">{groqNotice}</p> : null}

      <hr className="provider-divider" />

      <div className="provider-head">
        <h2>MyMemory</h2>
        <span className="badge ok">Always available</span>
      </div>

      <p className="muted small">{myMemoryStatusLine()}</p>
    </section>
  );
}

/** Errors arrive as serde-tagged JSON; show the message, never a raw object. */
function describe(reason: unknown): string {
  return typeof reason === "string" ? reason : JSON.stringify(reason);
}
