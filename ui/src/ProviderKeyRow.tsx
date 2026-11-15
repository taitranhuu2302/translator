import { useEffect, useRef, useSyncExternalStore } from "react";

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
    <>
      <div className="provider-head">
        <h2>{config.title}</h2>
        <span className={configured ? "badge ok" : "badge"}>
          {configured ? "Key saved" : "No key"}
        </span>
      </div>

      <p className="muted small">
        {state.statusProblem
          ? `The provider status could not be read: ${state.statusProblem}`
          : state.status
            ? config.statusLine(state.status)
            : "Checking the Keychain…"}
      </p>

      <label className="field">
        <span>API key</span>
        <input
          type="password"
          value={state.draft}
          onChange={(event) => current.setDraft(event.target.value)}
          placeholder={config.keyPlaceholder(configured)}
          autoComplete="off"
          spellCheck={false}
        />
      </label>

      {config.region ? (
        <label className="field">
          <span>Region (optional)</span>
          <input
            value={state.region}
            onChange={(event) => current.setRegion(event.target.value)}
            placeholder="e.g. westeurope — leave empty for a global resource"
            autoComplete="off"
            spellCheck={false}
          />
        </label>
      ) : null}

      <div className="actions">
        <button
          className="primary"
          onClick={() => void current.save()}
          disabled={state.busy || state.draft.trim().length === 0}
        >
          Save key
        </button>
        {configured ? (
          <button onClick={() => void current.clear()} disabled={state.busy}>
            Clear key
          </button>
        ) : null}
        {config.region ? (
          <button
            onClick={() => void current.saveRegion()}
            disabled={state.busy || !config.region.changed(settings, state.region)}
          >
            Save region
          </button>
        ) : null}
      </div>

      {state.notice ? <p className="muted small">{state.notice}</p> : null}
    </>
  );
}
