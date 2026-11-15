import {
  accessibilityGrantedNotice,
  isAccessibilityNotice,
  type Notice,
  type NoticeAction,
} from "./notice";

/**
 * The one way a notice reaches the screen.
 *
 * Both surfaces use this and pass only the class their layout needs; the words
 * — title, detail and the action's label — come from the notice itself, so a
 * surface cannot add a sentence of its own.
 */
export function NoticeView({
  notice,
  className,
  onAction,
}: {
  notice: Notice;
  className: string;
  onAction?: (action: NoticeAction) => void;
}) {
  const action = notice.action;
  return (
    <div className={`${className} ${notice.kind}`} role="alert">
      <div className="notice-text">
        <strong>{notice.title}</strong>
        <span className="muted small">{notice.detail}</span>
      </div>
      {action ? (
        <div className="actions">
          <button onClick={() => onAction?.(action)}>{action.label}</button>
        </div>
      ) : null}
    </div>
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
      <NoticeView
        className="notice"
        notice={accessibilityGrantedNotice(shortcut)}
      />
    );
  }
  return (
    <NoticeView
      className="notice"
      notice={notice}
      onAction={onAction}
    />
  );
}
