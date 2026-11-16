import { CheckCircle2 } from "lucide-react";
import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Kbd } from "@/components/ui/kbd";

import { formatShortcut } from "./format";
import { ipc, type ShortcutBindings } from "./ipc";

const SHORTCUTS: { key: keyof ShortcutBindings; name: string; action: string }[] = [
  {
    key: "quick_translate",
    name: "Quick Translate",
    action: "Translate the selection in a popup beside your cursor.",
  },
  {
    key: "replace",
    name: "Replace",
    action: "Translate the selection and overwrite it in place.",
  },
  {
    key: "read_aloud",
    name: "Read Aloud",
    action: "Hear the selection in the system voice.",
  },
  {
    key: "translate_window",
    name: "Translate Window",
    action: "Open this window with the selection already in it.",
  },
];

/**
 * The one-time welcome. It is a `Dialog`, so the keyboard stays inside it until
 * it is answered and returns to the window afterwards; Escape or the backdrop
 * puts it away for this session without marking it seen, and "Got it" is what
 * writes the flag.
 */
export function Onboarding({
  shortcuts,
  granted: grantedByApp,
  onDone,
}: {
  shortcuts: ShortcutBindings;
  granted: boolean;
  onDone: () => Promise<void>;
}) {
  const [open, setOpen] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [granted, setGranted] = useState(grantedByApp);
  const [grantProblem, setGrantProblem] = useState<string | null>(null);

  const finish = async () => {
    setSaving(true);
    setError(null);
    try {
      await onDone();
    } catch {
      setError("The onboarding could not be saved. Try again.");
      setSaving(false);
    }
  };

  // The window re-checks the permission whenever it comes back to the front,
  // so returning from System Settings updates the card without a click here.
  useEffect(() => {
    setGranted(grantedByApp);
  }, [grantedByApp]);

  // Asking macOS shows its own prompt; the answer is read back once, after the
  // prompt, so the surface never claims a permission it has not observed.
  const grantAccessibility = async () => {
    setGrantProblem(null);
    try {
      await ipc.requestAccessibility();
      setGranted(await ipc.accessibilityGranted());
    } catch {
      setGrantProblem("The Accessibility permission could not be checked.");
    }
  };

  const openPrivacySettings = async () => {
    setGrantProblem(null);
    try {
      await ipc.openAccessibilitySettings();
    } catch {
      setGrantProblem("System Settings could not be opened.");
    }
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent className="sm:max-w-md" showCloseButton={false}>
        <DialogHeader>
          <DialogTitle>Welcome to Translator</DialogTitle>
          <DialogDescription>
            Translator lives in the menu bar and works on whatever you select in
            another app. Select text anywhere, then use one of these.
          </DialogDescription>
        </DialogHeader>

        <ul className="flex flex-col gap-2.5">
          {SHORTCUTS.map((shortcut) => (
            <li className="flex items-start gap-3" key={shortcut.key}>
              <Kbd className="mt-0.5 shrink-0">
                {formatShortcut(shortcuts[shortcut.key])}
              </Kbd>
              <div className="min-w-0">
                <p className="text-sm text-foreground">{shortcut.name}</p>
                <p className="text-xs leading-relaxed text-muted-foreground">
                  {shortcut.action}
                </p>
              </div>
            </li>
          ))}
        </ul>

        <div className="flex flex-col gap-2.5 rounded-lg border border-border/60 bg-card/50 px-3 py-2.5">
          <p className="text-xs leading-relaxed text-muted-foreground">
            Reading your selection needs the Accessibility permission. macOS
            asks the first time a shortcut fires; you can also grant it now from
            System Settings → Privacy &amp; Security → Accessibility.
          </p>
          {granted ? (
            <p
              className="flex items-center gap-1.5 text-xs text-primary"
              role="status"
            >
              <CheckCircle2 aria-hidden="true" className="size-3.5" />
              Accessibility is granted.
            </p>
          ) : (
            <div className="flex flex-wrap items-center gap-2">
              <Button
                size="sm"
                variant="outline"
                onClick={() => {
                  void grantAccessibility();
                }}
              >
                Grant Accessibility…
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => {
                  void openPrivacySettings();
                }}
              >
                Open Privacy Settings
              </Button>
            </div>
          )}
          {grantProblem ? (
            <p className="text-xs text-destructive" role="alert">
              {grantProblem}
            </p>
          ) : null}
        </div>

        {error ? (
          <p className="text-xs text-destructive" role="alert">
            {error}
          </p>
        ) : null}

        <DialogFooter>
          <Button onClick={finish} disabled={saving}>
            {saving ? "Saving…" : "Got it"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
