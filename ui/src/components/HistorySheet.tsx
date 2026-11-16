import { HistoryIcon, SearchIcon } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";

import { filterHistory, historyModeLabel, relativeTime } from "../historyView";
import type { HistoryEntry } from "../ipc";
import { EmptyState } from "./EmptyState";

export const EMPTY_HISTORY =
  "Nothing here yet — translations and improvements you make will appear here.";

/**
 * History, beside the work rather than instead of it.
 *
 * The entries were read once when the sheet opened; the field filters them here
 * and never asks the engine anything. Choosing an entry restores it into the
 * panes and closes the sheet — the row's job is to be chosen, so it is a button.
 */
export function HistorySheet({
  open,
  onOpenChange,
  entries,
  query,
  error,
  onQueryChange,
  onOpen,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  entries: HistoryEntry[];
  query: string;
  error: string | null;
  onQueryChange: (value: string) => void;
  onOpen: (entry: HistoryEntry) => void;
}) {
  const visible = filterHistory(entries, query);
  const now = Math.floor(Date.now() / 1000);

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent
        side="right"
        className="w-[400px] gap-0 p-0 sm:max-w-[400px]"
      >
        <SheetHeader className="gap-1 border-b border-border/60 px-4 py-3.5 pr-12">
          <SheetTitle>History</SheetTitle>
          <SheetDescription className="text-xs">
            Kept on this Mac for 30 days
          </SheetDescription>
        </SheetHeader>

        <div className="border-b border-border/60 px-3 py-2.5">
          <div className="relative">
            <SearchIcon
              aria-hidden="true"
              className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground"
            />
            <Input
              type="search"
              aria-label="Search history"
              placeholder="Search either side…"
              className="pl-8"
              value={query}
              onChange={(event) => onQueryChange(event.target.value)}
              spellCheck={false}
              autoFocus
            />
          </div>
        </div>

        {error ? (
          <p role="alert" className="border-b border-border/60 px-4 py-2 text-xs text-destructive">
            {error}
          </p>
        ) : null}

        <div className="min-h-0 flex-1 overflow-y-auto">
          {visible.length === 0 ? (
            <EmptyState icon={HistoryIcon} title={EMPTY_HISTORY} />
          ) : (
            <ul>
              {visible.map((entry, index) => (
                <li key={`${entry.created_at}-${index}`}>
                  <button
                    type="button"
                    onClick={() => onOpen(entry)}
                    className="flex w-full flex-col gap-1 border-b border-border/40 px-4 py-3 text-left transition-colors outline-none hover:bg-muted/50 focus-visible:bg-muted/50 focus-visible:ring-2 focus-visible:ring-ring/40 focus-visible:ring-inset"
                  >
                    <span className="flex items-center gap-2">
                      <Badge
                        variant="outline"
                        className="border-border/60 bg-background/40 text-[0.625rem] leading-none text-muted-foreground"
                      >
                        {historyModeLabel(entry.mode)}
                      </Badge>
                      <span className="text-[0.6875rem] text-muted-foreground">
                        {relativeTime(entry.created_at, now)}
                      </span>
                    </span>
                    <span className="line-clamp-2 text-sm text-foreground">
                      {entry.source}
                    </span>
                    <span className="line-clamp-2 text-xs leading-relaxed text-muted-foreground">
                      {entry.result}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </SheetContent>
    </Sheet>
  );
}
