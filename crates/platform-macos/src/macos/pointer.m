#import "pointer.h"

#import <AppKit/AppKit.h>

// The cursor and the visible frame of the screen it is on, in AppKit's global
// point space (bottom-left origin at the primary screen's bottom-left corner).
// `Window::cursor_position()` cannot be used: tao flips Y with the primary
// display's *pixel* height while `NSEvent::mouseLocation` is in points, and then
// scales by the primary's backing factor even when the cursor is elsewhere.
tl_screen_info tl_screen_under_cursor(void) {
    tl_screen_info info = {0};

    NSArray<NSScreen *> *screens = [NSScreen screens];
    if (screens.count == 0) {
        return info;
    }

    NSPoint cursor = [NSEvent mouseLocation];

    NSScreen *chosen = nil;
    for (NSScreen *screen in screens) {
        if (NSPointInRect(cursor, screen.frame)) {
            chosen = screen;
            break;
        }
    }
    // The pointer can sit on the seam between two screens. Fall back to the
    // screen with the key window rather than guessing at an index.
    if (chosen == nil) {
        chosen = [NSScreen mainScreen];
    }
    if (chosen == nil) {
        return info;
    }

    NSScreen *primary = screens.firstObject;  // documented: the first is primary
    NSRect visible = chosen.visibleFrame;

    info.ok = true;
    info.cursor = (tl_point){cursor.x, cursor.y};
    info.visible = (tl_rect){
        visible.origin.x,
        visible.origin.y,
        visible.size.width,
        visible.size.height,
    };
    info.scale = chosen.backingScaleFactor;
    info.desktop_height = primary.frame.size.height;
    info.desktop_pixels = primary.frame.size.height * primary.backingScaleFactor;
    return info;
}
