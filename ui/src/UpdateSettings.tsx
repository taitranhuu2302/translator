/**
 * The Updates section of the Settings sheet.
 *
 * The shell owns the update: it checks at launch and every six hours, downloads
 * silently, and holds the result until the user restarts. This view only draws
 * that state and asks the shell to move it — it never decides whether a version
 * is newer and never restarts on its own.
 * (docs/adr/0009-self-signed-distribution-and-in-app-updates.md)
 */
import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";

import {
  ipc,
  onUpdateStatus,
  type Settings,
  type UpdateState,
  type UpdateStatus,
} from "./ipc";
import { settingsWriter } from "./settingsWrite";

/** The one sentence for a check that never reached the shell. */
const STATUS_READ_FAILED = "The update state could not be read.";
const CHECK_FAILED = "The update check could not be started.";
const RESTART_FAILED = "Neris could not restart into the update.";

export function UpdateSettings({
  settings,
  onSettingsSaved,
}: {
  settings: Settings;
  onSettingsSaved: (settings: Settings) => void;
}) {
  const [status, setStatus] = useState<UpdateStatus | null>(null);
  const [readProblem, setReadProblem] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  // The sheet can open after a check already finished, so the current state is
  // read once; every later move arrives as an event.
  useEffect(() => {
    let cancelled = false;
    ipc
      .updateStatus()
      .then((current) => {
        if (!cancelled) setStatus(current);
      })
      .catch(() => {
        if (!cancelled) setReadProblem(STATUS_READ_FAILED);
      });
    const unlisten = onUpdateStatus((next) => setStatus(next));
    return () => {
      cancelled = true;
      void unlisten
        .then((stop) => stop())
        .catch(() => {
          // The listener never attached, so there is nothing to stop.
        });
    };
  }, []);

  const checkNow = async () => {
    setNotice(null);
    setProblem(null);
    try {
      const next = await ipc.checkForUpdates();
      setStatus(next);
      if (next.state === "up_to_date") setNotice("Neris is up to date.");
      if (next.state === "failed") setProblem(next.problem ?? CHECK_FAILED);
    } catch {
      setProblem(CHECK_FAILED);
    }
  };

  const restart = async () => {
    setProblem(null);
    try {
      // On success the app quits here; only a refusal gets to answer.
      await ipc.restartToUpdate();
    } catch {
      setProblem(RESTART_FAILED);
    }
  };

  const saveAutoCheck = async (enabled: boolean) => {
    setBusy(true);
    setNotice(null);
    setProblem(null);
    try {
      const outcome = await settingsWriter.change({
        kind: "auto_check_updates",
        enabled,
      });
      if (outcome.mirror === null) {
        setProblem(outcome.problem);
        return;
      }
      onSettingsSaved(outcome.mirror.settings);
      setNotice(outcome.notice);
    } finally {
      setBusy(false);
    }
  };

  const state: UpdateState | null = status?.state ?? null;
  const working = state === "checking" || state === "downloading";

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between gap-3">
        <div className="min-w-0">
          <p className="text-sm text-foreground">
            Version {status?.current_version ?? "…"}
          </p>
          <p className="text-xs leading-relaxed text-muted-foreground">
            {statusLine(status, state)}
          </p>
        </div>
        <Button
          size="xs"
          variant="outline"
          onClick={() => {
            void checkNow();
          }}
          disabled={busy || state === null || state === "disabled" || working}
        >
          {working ? "Checking…" : "Check now"}
        </Button>
      </div>

      {/* A ready update is the one state with a step the user can take from
          here, and the menu bar offers the same step. */}
      {state === "ready" && status?.available_version ? (
        <div className="flex flex-wrap items-center gap-2 rounded-lg border border-border/70 bg-card/60 px-3 py-2.5">
          <p className="min-w-0 flex-1 text-xs text-foreground">
            Version {status.available_version} is ready.
          </p>
          <Button
            size="xs"
            onClick={() => {
              void restart();
            }}
          >
            Restart to Update
          </Button>
        </div>
      ) : null}

      <div className="flex items-center justify-between gap-3">
        <Label htmlFor="auto-check-updates" className="font-normal">
          Check for updates automatically
        </Label>
        <Switch
          id="auto-check-updates"
          checked={settings.auto_check_updates}
          disabled={busy || state === "disabled"}
          onCheckedChange={(checked) => {
            void saveAutoCheck(checked);
          }}
        />
      </div>

      {readProblem ? (
        <p className="text-xs text-destructive" role="alert">
          {readProblem}
        </p>
      ) : null}
      {problem ?? (state === "failed" ? status?.problem : null) ? (
        <p className="text-xs text-destructive" role="alert">
          {problem ?? status?.problem}
        </p>
      ) : null}
      {notice ? (
        <p className="text-xs text-muted-foreground" aria-live="polite">
          {notice}
        </p>
      ) : null}
    </div>
  );
}

/** What the section says about the shell's state. Each state gets one sentence;
 * `null` while the first read is still in flight, so the row does not claim
 * anything it does not know. */
function statusLine(
  status: UpdateStatus | null,
  state: UpdateState | null,
): string | null {
  switch (state) {
    case "disabled":
      return "This build cannot update itself. A newer version arrives as a downloaded Release.";
    case "idle":
      return status?.problem
        ? status.problem
        : "Neris checks at launch and every six hours.";
    case "checking":
      return "Looking for a newer version…";
    case "downloading":
      return status?.available_version
        ? `Downloading version ${status.available_version}…`
        : "Downloading the update…";
    case "up_to_date":
      return "Neris is up to date.";
    case "ready":
      return "The update is downloaded and waiting for a restart.";
    case "failed":
      return status?.problem ?? "The update check did not finish.";
    default:
      return null;
  }
}
