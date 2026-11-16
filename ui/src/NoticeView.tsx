import {
  AlertCircle,
  AlertTriangle,
  CheckCircle2,
  CloudOff,
  WifiOff,
  type LucideIcon,
} from "lucide-react";

import {
  Alert,
  AlertDescription,
  AlertTitle,
} from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

import {
  accessibilityGrantedNotice,
  isAccessibilityNotice,
  type Notice,
  type NoticeAction,
} from "./notice";

/** One icon and one tint per cause, so a missing grant and a dead network do
 * not look like the same failure. The words never change here. */
const KIND: Record<
  Notice["kind"],
  { icon: LucideIcon; className: string; titleClassName?: string }
> = {
  capture: { icon: AlertTriangle, className: "border-border/70 bg-card/60" },
  offline: { icon: WifiOff, className: "border-border/70 bg-card/60" },
  provider: { icon: CloudOff, className: "border-border/70 bg-card/60" },
  error: {
    icon: AlertCircle,
    className: "border-destructive/40 bg-destructive/10",
  },
  ok: {
    icon: CheckCircle2,
    className: "border-primary/40 bg-primary/10",
    titleClassName: "text-primary",
  },
};

/**
 * The one way a notice reaches the screen.
 *
 * Both surfaces use this and pass only the layout class they need; the words —
 * title, detail and the action's label — come from the notice itself, so a
 * surface cannot add a sentence of its own.
 */
export function NoticeView({
  notice,
  className,
  onAction,
}: {
  notice: Notice;
  className?: string;
  onAction?: (action: NoticeAction) => void;
}) {
  const { icon: Icon, className: kindClassName, titleClassName } = KIND[notice.kind];
  const action = notice.action;

  return (
    <Alert
      className={cn(
        kindClassName,
        "gap-1",
        titleClassName ? "*:data-[slot=alert-title]:text-primary" : undefined,
        className,
      )}
    >
      <Icon aria-hidden="true" />
      <AlertTitle>{notice.title}</AlertTitle>
      <AlertDescription className="text-xs leading-relaxed">
        <p>{notice.detail}</p>
        {/* The action sits under the words, not over them: every surface this
            draws on is narrow enough that a floating button would cover the
            title it belongs to. */}
        {action ? (
          <Button
            size="xs"
            variant="outline"
            className="mt-2.5"
            onClick={() => onAction?.(action)}
          >
            {action.label}
          </Button>
        ) : null}
      </AlertDescription>
    </Alert>
  );
}

/**
 * The Translate Window's capture notice. A grant can land while the window is
 * open — returning from System Settings re-checks it — so the failure that
 * brought the user there is replaced by the model's resolved notice. The
 * surface only chooses which notice applies; both are worded in the model.
 */
export function CaptureNoticeView({
  notice,
  granted,
  shortcut,
  onAction,
}: {
  notice: Notice;
  granted: boolean | null;
  shortcut: string;
  onAction?: (action: NoticeAction) => void;
}) {
  if (granted === true && isAccessibilityNotice(notice)) {
    return (
      <NoticeView notice={accessibilityGrantedNotice(shortcut)} />
    );
  }
  return <NoticeView notice={notice} onAction={onAction} />;
}
