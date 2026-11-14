import { ipc } from "./ipc";
import type { NoticeAction } from "./notice";

/**
 * The one place a notice's action becomes a command.
 *
 * Both surfaces hand this to `NoticeView`, so an action is wired once here and
 * worded once in the model — neither surface knows what the button does.
 */
export function performNoticeAction(action: NoticeAction): void {
  switch (action.id) {
    case "open_accessibility_settings":
      void ipc.openAccessibilitySettings().catch(() => {
        // System Settings may simply not open; the notice stays as it is.
      });
  }
}
