import { NerisMark } from "@/components/NerisMark";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";

import type { Settings } from "../ipc";
import { noticeForSettingsFailure } from "../notice";
import { NoticeView } from "../NoticeView";
import { SettingsPane } from "../SettingsPane";

/**
 * Settings, beside the work rather than instead of it.
 *
 * The sheet is chrome only: `SettingsPane` owns every control and every write,
 * and scrolls on its own inside a window that never moves. A sheet can still be
 * opened before the settings arrive, or after they failed to: it says which,
 * instead of refusing to open and leaving the button looking broken.
 */
export function SettingsSheet({
  open,
  onOpenChange,
  settings,
  error,
  onSettingsSaved,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  settings: Settings | null;
  error: string | null;
  onSettingsSaved: (settings: Settings) => void;
}) {
  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent
        side="right"
        className="w-[440px] gap-0 p-0 sm:max-w-[440px]"
      >
        <SheetHeader className="gap-1 border-b border-border/60 px-4 py-3.5 pr-12">
          <SheetTitle className="flex items-center gap-2">
            <NerisMark className="h-4 w-4 text-primary" />
            Settings
          </SheetTitle>
          <SheetDescription className="text-xs">
            Saved on this Mac, never synced
          </SheetDescription>
        </SheetHeader>

        <div className="min-h-0 flex-1 overflow-y-auto">
          {settings ? (
            <SettingsPane
              settings={settings}
              onSettingsSaved={onSettingsSaved}
            />
          ) : error ? (
            <div className="px-4 py-5">
              <NoticeView notice={noticeForSettingsFailure(error)} />
            </div>
          ) : (
            <p className="px-4 py-5 text-sm text-muted-foreground">
              Loading settings…
            </p>
          )}
        </div>
      </SheetContent>
    </Sheet>
  );
}
