#ifndef TRANSLATOR_POINTER_H
#define TRANSLATOR_POINTER_H

#include <stdbool.h>

typedef struct {
    double x;
    double y;
} tl_point;

typedef struct {
    double x;
    double y;
    double width;
    double height;
} tl_rect;

typedef struct {
    /** Whether the query found a screen. Every other field is zero when false. */
    bool ok;
    /** The cursor, in AppKit's global points: bottom-left origin, primary screen. */
    tl_point cursor;
    /** The visible frame of the screen the cursor is on, in the same points. */
    tl_rect visible;
    /** The backing scale of that screen (1.0 or 2.0). */
    double scale;
    /** The primary screen's height in points: the desktop's top edge. */
    double desktop_height;
    /**
     * The primary screen's height in device pixels. tao flips Y with
     * `CGDisplay::main().pixels_high()`, so window positions are exact only when
     * the conversion uses this value, not the point height.
     */
    double desktop_pixels;
} tl_screen_info;

tl_screen_info tl_screen_under_cursor(void);

#endif
