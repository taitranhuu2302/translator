import type { Settings } from "./ipc";
import { AZURE_ROW, GROQ_ROW } from "./providerKey";
import { myMemoryStatusLine } from "./providerStatus";
import { ProviderKeyRow } from "./ProviderKeyRow";

/**
 * The provider rows of the Settings surface. Azure and Groq are two
 * configurations of the one key-row module, and MyMemory — the keyless
 * fallback — is always there beside them. Saving or clearing a key returns the
 * refreshed status, so a new key stops showing as skipped immediately.
 */
export function ProviderSettings({
  settings,
  onSettingsSaved,
}: {
  settings: Settings;
  onSettingsSaved: (settings: Settings) => void;
}) {
  return (
    <section className="provider-settings">
      <ProviderKeyRow
        config={AZURE_ROW}
        settings={settings}
        onSettingsSaved={onSettingsSaved}
      />

      <hr className="provider-divider" />

      <ProviderKeyRow
        config={GROQ_ROW}
        settings={settings}
        onSettingsSaved={onSettingsSaved}
      />

      <hr className="provider-divider" />

      <div className="provider-head">
        <h2>MyMemory</h2>
        <span className="badge ok">Always available</span>
      </div>

      <p className="muted small">{myMemoryStatusLine()}</p>
    </section>
  );
}
