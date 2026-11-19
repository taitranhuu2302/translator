import { Loader2 } from "lucide-react";
import { useEffect, useState } from "react";

import { ipc, onReplaceHudPhase, type ReplacePhase } from "./ipc";
import { REPLACE_PHASE_COPY } from "./statusCopy";

/**
 * The Replace HUD's page: one line naming the step the shortcut is on.
 *
 * The shell shows this window only when a Replace outlives the 120 ms grace
 * period, and hides it the moment the Replace ends; the page decides nothing
 * about visibility. It draws nothing until it knows a step, so a window that
 * was shown for a Replace that already ended stays empty rather than showing
 * the previous run's words.
 */
export function Hud() {
  const [phase, setPhase] = useState<ReplacePhase | null>(null);

  useEffect(() => {
    let active = true;
    let unlisten = () => {};

    void onReplaceHudPhase((next) => {
      if (active) setPhase(next);
    }).then((stop) => {
      if (active) unlisten = stop;
      else stop();
    });

    // The shell emits the step as it appears, but this page may still have been
    // loading then. Claiming the current step on mount closes that gap.
    void ipc
      .replaceHudPhase()
      .then((current) => {
        if (active && current) setPhase(current);
      })
      .catch(() => {
        // A HUD that cannot read the step has nothing truthful to draw.
      });

    return () => {
      active = false;
      unlisten();
    };
  }, []);

  if (!phase) return null;

  return (
    <main className="flex h-full items-center p-2.5">
      <ReplaceHudCard phase={phase} />
    </main>
  );
}

/**
 * The capsule itself, apart from the IPC that feeds it, so the design preview
 * can draw every step without a shell.
 */
export function ReplaceHudCard({ phase }: { phase: ReplacePhase }) {
  return (
    <p
      role="status"
      className="flex w-full items-center gap-2 rounded-lg border border-border/40 bg-card px-3 py-1.5 text-[0.8125rem] text-muted-foreground shadow-lg"
    >
      <Loader2 aria-hidden="true" className="size-3.5 shrink-0 animate-spin" />
      <span className="truncate">{REPLACE_PHASE_COPY[phase]}</span>
    </p>
  );
}
