import {
  ArrowLeftRightIcon,
  CheckIcon,
  CopyIcon,
  LoaderIcon,
  SparklesIcon,
  TextCursorInputIcon,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";

import { providerLabel } from "../format";
import { partLabel, type ImproveResult } from "../improveWindow";
import { directionLabel } from "../languageLabel";
import type { Direction, ImprovePart, Mode } from "../ipc";
import { NoticeView } from "../NoticeView";
import type { TranslateResult } from "../translateWindow";
import { EmptyState } from "./EmptyState";

const IMPROVE_PARTS: ImprovePart[] = ["corrected", "better"];

/**
 * The two panes, in one frame and two roles.
 *
 * Source is an editable card; the result is the same card read-only, with its
 * actions in its header instead of a row at the bottom. Both modes draw this —
 * only the labels, the placeholder and the result body change.
 */
export function TranslatePanes({
  mode,
  text,
  onTextChange,
  onSubmit,
  result,
  liveDirection,
  improvement,
  copied,
  copiedPart,
  onCopy,
  onCopyPart,
  onSwap,
  canSwap,
  shortcut,
}: {
  mode: Mode;
  text: string;
  onTextChange: (value: string) => void;
  onSubmit: () => void;
  result: TranslateResult;
  /** The direction the next Translate would run in, or null when the text has
   * no guess to show. It labels the Source pane and decides nothing. */
  liveDirection: Direction | null;
  improvement: ImproveResult;
  copied: boolean;
  copiedPart: ImprovePart | null;
  onCopy: () => void;
  onCopyPart: (part: ImprovePart) => void;
  onSwap: () => void;
  canSwap: boolean;
  shortcut: string;
}) {
  const isImprove = mode === "improve";
  const busy = isImprove
    ? improvement.phase === "improving"
    : result.phase === "translating";
  const actionLabel = isImprove ? "Improve" : "Translate";
  const canCopy = !isImprove && result.phase === "translated";
  const improveParts =
    improvement.phase === "improving" || improvement.phase === "improved"
      ? improvement
      : null;

  return (
    <div className="grid min-h-0 flex-1 grid-cols-2 gap-4 p-4">
      <section className="@container flex min-h-0 flex-col overflow-hidden rounded-xl border border-border/70 bg-card shadow-sm transition-[border-color,box-shadow] focus-within:border-ring/50 focus-within:ring-3 focus-within:ring-ring/15">
        <PaneHeader label="Source">
          {/* The live Detection. It sits in the header, not in an aria-live
              region: it changes with every keystroke and is advice, not an
              announcement. An em dash is the honest answer when the guess has
              nothing to name. */}
          {isImprove ? null : (
            <span className="ml-auto min-w-0 truncate rounded-full bg-muted/70 px-2 py-0.5 text-[0.625rem] text-muted-foreground">
              {liveDirection ? directionLabel(liveDirection) : "—"}
            </span>
          )}
          {text.length > 0 ? (
            <span className="shrink-0 text-[0.6875rem] tabular-nums text-muted-foreground/80">
              {text.length} characters
            </span>
          ) : null}
        </PaneHeader>

        <Textarea
          aria-label={isImprove ? "English text to improve" : "Text to translate"}
          className="min-h-0 flex-1 resize-none rounded-none border-0 bg-transparent px-3.5 py-3 text-[0.9375rem] leading-relaxed shadow-none focus-visible:ring-0 dark:bg-transparent"
          value={text}
          onChange={(event) => onTextChange(event.target.value)}
          onKeyDown={(event) => {
            // Enter acts; Shift+Enter is the newline the editor needs.
            if (event.key === "Enter" && !event.shiftKey) {
              event.preventDefault();
              onSubmit();
            }
          }}
          placeholder={
            isImprove
              ? "Type an English sentence to correct and improve."
              : `Select text in any app and press ${shortcut}, or type here.`
          }
          spellCheck={false}
          autoFocus
        />

        <footer className="flex shrink-0 items-center gap-3 border-t border-border/60 px-3.5 py-2.5">
          <Button onClick={onSubmit} className="min-w-24">
            {busy ? <LoaderIcon className="animate-spin" /> : null}
            {actionLabel}
          </Button>
          {/* At the window's narrowest the button and this hint cannot both be
              whole, so the hint steps aside rather than ending in an ellipsis:
              it is advice, and the keys are named in the footer anyway. */}
          <span className="hidden truncate text-xs text-muted-foreground @[22rem]:inline">
            {text.trim().length === 0
              ? isImprove
                ? "Nothing to improve — the window opens empty."
                : "Nothing selected — the window opens empty."
              : `Enter ${isImprove ? "improves" : "translates"} · Shift+Enter adds a line`}
          </span>
        </footer>
      </section>

      <section className="flex min-h-0 flex-col overflow-hidden rounded-xl border border-border/70 bg-card shadow-sm">
        <PaneHeader label={isImprove ? "Improved" : "Translation"}>
          {result.phase === "translated" ? (
            <span className="shrink-0 rounded-full bg-muted/70 px-2 py-0.5 text-[0.625rem] whitespace-nowrap text-muted-foreground">
              {directionLabel(result.translation)} ·{" "}
              {providerLabel(result.translation.provider)}
            </span>
          ) : null}

          {isImprove ? null : (
            <div className="ml-auto flex items-center gap-0.5">
              <Button
                variant="ghost"
                size="xs"
                onClick={onSwap}
                disabled={!canSwap}
              >
                <ArrowLeftRightIcon />
                Swap
              </Button>
              <Button
                variant="ghost"
                size="xs"
                onClick={onCopy}
                disabled={!canCopy}
              >
                {copied ? <CheckIcon /> : <CopyIcon />}
                {copied ? "Copied" : "Copy"}
              </Button>
            </div>
          )}
        </PaneHeader>

        <div className="min-h-0 flex-1 overflow-y-auto px-3.5 py-3">
          {isImprove ? (
            <>
              {improvement.phase === "empty" ? (
                <EmptyState
                  icon={SparklesIcon}
                  title="Press Enter to improve."
                  detail="The sentence comes back corrected, then written more naturally."
                />
              ) : null}

              {improveParts ? (
                <div className="flex flex-col gap-4">
                  {IMPROVE_PARTS.map((part) => (
                    <section key={part}>
                      <div className="mb-1.5 flex items-center justify-between gap-2">
                        <h3 className="text-[0.6875rem] font-medium uppercase tracking-[0.08em] text-muted-foreground">
                          {partLabel(part)}
                        </h3>
                        <Button
                          variant="ghost"
                          size="xs"
                          onClick={() => onCopyPart(part)}
                          disabled={improveParts[part].length === 0}
                        >
                          {copiedPart === part ? (
                            <CheckIcon />
                          ) : (
                            <CopyIcon />
                          )}
                          {copiedPart === part ? "Copied" : "Copy"}
                        </Button>
                      </div>
                      {improveParts[part].length > 0 ? (
                        <p
                          className="whitespace-pre-wrap text-[0.9375rem] leading-relaxed text-foreground/90"
                          aria-label={partLabel(part)}
                        >
                          {improveParts[part]}
                        </p>
                      ) : (
                        <AnswerSkeleton />
                      )}
                    </section>
                  ))}
                </div>
              ) : null}

              {improvement.phase === "failed" ? (
                <NoticeView notice={improvement} />
              ) : null}
            </>
          ) : (
            <>
              {result.phase === "empty" ? (
                <EmptyState
                  icon={TextCursorInputIcon}
                  title="Press Enter to translate."
                  detail="The result can be selected in part."
                />
              ) : null}

              {result.phase === "translating" ? (
                <div>
                  <AnswerSkeleton />
                  <p role="status" className="mt-3 text-xs text-muted-foreground">
                    Translating…
                  </p>
                </div>
              ) : null}

              {result.phase === "translated" ? (
                <p
                  className="whitespace-pre-wrap text-[0.9375rem] leading-relaxed text-foreground/90"
                  aria-label="Translation"
                >
                  {result.translation.text}
                </p>
              ) : null}

              {result.phase === "failed" ? (
                <NoticeView notice={result} />
              ) : null}
            </>
          )}
        </div>
      </section>
    </div>
  );
}

function PaneHeader({
  label,
  children,
}: {
  label: string;
  children?: React.ReactNode;
}) {
  return (
    <header className="flex h-10 shrink-0 items-center gap-2 border-b border-border/60 px-3.5">
      <span className="shrink-0 text-[0.6875rem] font-medium uppercase tracking-[0.08em] text-muted-foreground">
        {label}
      </span>
      {children}
    </header>
  );
}

/** A skeleton in the shape of a paragraph, so the answer does not move the
 * layout when it lands. */
function AnswerSkeleton({ className }: { className?: string }) {
  return (
    <div className={cn("space-y-2", className)} aria-hidden="true">
      <Skeleton className="h-3.5 w-[94%] bg-foreground/10" />
      <Skeleton className="h-3.5 w-[88%] bg-foreground/10" />
      <Skeleton className="h-3.5 w-[91%] bg-foreground/10" />
      <Skeleton className="h-3.5 w-[52%] bg-foreground/10" />
    </div>
  );
}
