import { Badge } from "@/components/ui/badge";

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
    <div className="flex flex-col gap-2">
      <ProviderKeyRow
        config={AZURE_ROW}
        settings={settings}
        onSettingsSaved={onSettingsSaved}
      />

      <ProviderKeyRow
        config={GROQ_ROW}
        settings={settings}
        onSettingsSaved={onSettingsSaved}
      />

      <div className="flex flex-col gap-2 rounded-lg border border-border/60 bg-card/50 px-3 py-3">
        <div className="flex items-center justify-between gap-2">
          <h4 className="text-sm text-foreground">MyMemory</h4>
          <Badge
            className="border-transparent bg-primary/15 text-[0.625rem] leading-none text-primary"
            variant="outline"
          >
            Always available
          </Badge>
        </div>
        <p className="text-xs leading-relaxed text-muted-foreground">
          {myMemoryStatusLine()}
        </p>
      </div>
    </div>
  );
}
