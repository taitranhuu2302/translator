import type { LucideIcon } from "lucide-react";

import { cn } from "@/lib/utils";

/**
 * The one composed empty state: a muted icon, a sentence, and an optional
 * second line. Panes, History and the settings sheets all draw this instead of
 * a lone grey paragraph, so "nothing here yet" looks the same everywhere.
 */
export function EmptyState({
  icon: Icon,
  title,
  detail,
  className,
}: {
  icon: LucideIcon;
  title: string;
  detail?: string;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "flex h-full flex-col items-center justify-center gap-2 px-6 py-8 text-center",
        className,
      )}
    >
      <Icon
        aria-hidden="true"
        strokeWidth={1.5}
        className="size-5 text-muted-foreground/70"
      />
      <p className="text-sm text-foreground/80">{title}</p>
      {detail ? (
        <p className="max-w-xs text-xs leading-relaxed text-muted-foreground">
          {detail}
        </p>
      ) : null}
    </div>
  );
}
