import { useEffect, useRef, useSyncExternalStore } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

import { ipc, type Settings } from "./ipc";
import {
  createProviderKeyRow,
  isConfigured,
  type ProviderKeyRowConfig,
  type ProviderKeyRowModel,
} from "./providerKey";
import { settingsWriter } from "./settingsWrite";

/**
 * One provider's key row, drawn from its configuration and its own state.
 *
 * The page holds none of the row's draft, busy state or notice: each mounted
 * row owns them, and `providerKey.ts` owns the rules that change them. A
 * second provider is one more configuration, not another copy of this markup.
 */
export function ProviderKeyRow({
  config,
  settings,
  onSettingsSaved,
}: {
  config: ProviderKeyRowConfig;
  settings: Settings;
  onSettingsSaved: (settings: Settings) => void;
}) {
  // The row is created once per mount. The latest callback is kept here so a
  // region save reaches the page's state setter without rebuilding the row.
  const saved = useRef(onSettingsSaved);
  saved.current = onSettingsSaved;

  const row = useRef<ProviderKeyRowModel | null>(null);
  if (row.current === null) {
    row.current = createProviderKeyRow(
      config,
      {
        readStatus: () => ipc.providerStatus(),
        writer: settingsWriter,
        settingsSaved: (next) => saved.current(next),
      },
      { region: config.region ? config.region.read(settings) : "" },
    );
  }
  const current = row.current;
  const state = useSyncExternalStore(current.subscribe, () => current.state);

  useEffect(() => {
    void current.load();
  }, [current]);

  // Settings can be reloaded elsewhere; the region field follows the saved
  // value without touching the key draft or the last notice.
  const savedRegion = config.region ? config.region.read(settings) : "";
  useEffect(() => {
    current.setRegion(savedRegion);
  }, [current, savedRegion]);

  const configured = isConfigured(config.provider, state.status);

  return (
    <div className="flex flex-col gap-2.5 rounded-lg border border-border/60 bg-card/50 px-3 py-3">
      <div className="flex items-center justify-between gap-2">
        <h4 className="text-sm text-foreground">{config.title}</h4>
        <Badge
          variant={configured ? "secondary" : "outline"}
          className="text-[0.625rem] leading-none"
        >
          {configured ? "Key saved" : "No key"}
        </Badge>
      </div>

      <p className="text-xs leading-relaxed text-muted-foreground">
        {state.statusProblem
          ? state.statusProblem
          : state.status
            ? config.statusLine(state.status)
            : "Checking the Keychain…"}
      </p>

      <div className="flex flex-col gap-1.5">
        <Label
          htmlFor={`${config.provider}-key`}
          className="text-xs font-normal text-muted-foreground"
        >
          API key
        </Label>
        <Input
          id={`${config.provider}-key`}
          type="password"
          value={state.draft}
          onChange={(event) => current.setDraft(event.target.value)}
          placeholder={config.keyPlaceholder(configured)}
          autoComplete="off"
          spellCheck={false}
        />
      </div>

      {config.region ? (
        <div className="flex flex-col gap-1.5">
          <Label
            htmlFor={`${config.provider}-region`}
            className="text-xs font-normal text-muted-foreground"
          >
            Region (optional)
          </Label>
          <Input
            id={`${config.provider}-region`}
            value={state.region}
            onChange={(event) => current.setRegion(event.target.value)}
            placeholder="e.g. westeurope — leave empty for a global resource"
            autoComplete="off"
            spellCheck={false}
          />
        </div>
      ) : null}

      <div className="flex flex-wrap items-center gap-2">
        <Button
          size="sm"
          onClick={() => void current.save()}
          disabled={state.busy || state.draft.trim().length === 0}
        >
          Save key
        </Button>
        {configured ? (
          <Button
            size="sm"
            variant="ghost"
            onClick={() => void current.clear()}
            disabled={state.busy}
          >
            Clear key
          </Button>
        ) : null}
        {config.region ? (
          <Button
            size="sm"
            variant="ghost"
            onClick={() => void current.saveRegion()}
            disabled={
              state.busy || !config.region.changed(settings, state.region)
            }
          >
            Save region
          </Button>
        ) : null}
      </div>

      {state.notice ? (
        <p className="text-xs text-muted-foreground">{state.notice}</p>
      ) : null}
    </div>
  );
}
