//! Where the Quick Translate popup goes.
//!
//! Two separate jobs live here:
//!
//! * `screen_under_cursor` asks AppKit for the cursor and the visible frame of
//!   the screen it is on, in points. Tauri's `Window::cursor_position` is not
//!   used because it flips Y with the primary display's pixel height and scales
//!   by the primary's factor, which is wrong on Retina and mixed-DPI desktops.
//! * `place_popup` and `to_physical` are pure arithmetic, tested without a
//!   window. They own the bottom-edge flip and the clamp to the screen, and the
//!   conversion into the physical top-left coordinates Tauri's window API takes.

/// A point in AppKit's global space: points, bottom-left origin at the primary
/// screen's bottom-left corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Everything placement needs, captured in one AppKit query.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenUnderCursor {
    pub cursor: Point,
    pub visible: Rect,
    pub scale: f64,
    /// The primary screen's height in points: the top edge of the desktop.
    pub desktop_height: f64,
    /// The primary screen's height in device pixels, which is the flip constant
    /// tao uses when it turns a Tauri position into an AppKit frame point.
    pub desktop_pixels: f64,
}

#[cfg(target_os = "macos")]
mod bridge {
    use super::{Point, Rect, ScreenUnderCursor};

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub(super) struct RawPoint {
        pub x: f64,
        pub y: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub(super) struct RawRect {
        pub x: f64,
        pub y: f64,
        pub width: f64,
        pub height: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub(super) struct RawScreenInfo {
        pub ok: bool,
        pub cursor: RawPoint,
        pub visible: RawRect,
        pub scale: f64,
        pub desktop_height: f64,
        pub desktop_pixels: f64,
    }

    extern "C" {
        pub(super) fn tl_screen_under_cursor() -> RawScreenInfo;
    }

    pub(super) fn query() -> Option<ScreenUnderCursor> {
        // SAFETY: the bridge returns a plain C struct and reads only AppKit
        // state. It is called on the main thread by the shell.
        let raw = unsafe { tl_screen_under_cursor() };
        if !raw.ok {
            return None;
        }
        Some(ScreenUnderCursor {
            cursor: Point {
                x: raw.cursor.x,
                y: raw.cursor.y,
            },
            visible: Rect {
                x: raw.visible.x,
                y: raw.visible.y,
                width: raw.visible.width,
                height: raw.visible.height,
            },
            scale: raw.scale,
            desktop_height: raw.desktop_height,
            desktop_pixels: raw.desktop_pixels,
        })
    }
}

/// The cursor and the screen it is on, in AppKit points. `None` only when
/// AppKit reports no screens at all.
pub fn screen_under_cursor() -> Option<ScreenUnderCursor> {
    #[cfg(target_os = "macos")]
    {
        bridge::query()
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

/// The popup's top-left corner in points, measured from the top-left of the
/// desktop. It opens below and to the right of the cursor; near the bottom edge
/// it flips above the cursor, and it is always clamped inside `visible`.
pub fn place_popup(
    cursor: Point,
    size: Size,
    visible: Rect,
    desktop_height: f64,
    gap: f64,
) -> Point {
    let visible_top = desktop_height - (visible.y + visible.height);
    let visible_bottom = desktop_height - visible.y;
    let cursor_from_top = desktop_height - cursor.y;

    let x = cursor.x + gap;
    let mut top = cursor_from_top + gap;

    // The popup prefers to hang below the cursor; flip it above when the bottom
    // edge would fall off the screen. `to_physical` uses the same top-left form.
    if top + size.height > visible_bottom {
        top = cursor_from_top - gap - size.height;
    }

    let furthest_right = (visible.x + visible.width - size.width).max(visible.x);
    let furthest_down = (visible_bottom - size.height).max(visible_top);
    Point {
        x: x.clamp(visible.x, furthest_right),
        y: top.clamp(visible_top, furthest_down),
    }
}

/// Convert the top-left point from `place_popup` into the `PhysicalPosition`
/// Tauri's `set_position` expects.
///
/// Tauri's macOS backend converts a physical position back to a logical point
/// and then sets the AppKit frame point as `main_pixels_high - y`, so the flip
/// constant must be the primary screen's pixel height. `scale` is the *window's
/// current* backing scale, which is the factor that backend divides by.
pub fn to_physical(
    top_left: Point,
    desktop_height: f64,
    desktop_pixels: f64,
    window_scale: f64,
) -> Point {
    Point {
        x: top_left.x * window_scale,
        y: (desktop_pixels - desktop_height + top_left.y) * window_scale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DESKTOP_HEIGHT: f64 = 1080.0;
    const GAP: f64 = 8.0;
    const POPUP: Size = Size {
        width: 420.0,
        height: 240.0,
    };

    fn primary_visible() -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1055.0, // the menu bar takes the rest
        }
    }

    fn secondary_visible() -> Rect {
        // The second display from the research notes: 1512×982 points at 2x,
        // placed to the right of and slightly above the main display.
        Rect {
            x: 1920.0,
            y: 98.0,
            width: 1512.0,
            height: 982.0,
        }
    }

    #[test]
    fn a_popup_opens_below_and_to_the_right_of_the_cursor() {
        let placed = place_popup(
            Point { x: 500.0, y: 500.0 },
            POPUP,
            primary_visible(),
            DESKTOP_HEIGHT,
            GAP,
        );
        assert_eq!(
            placed,
            Point {
                x: 508.0,
                y: 588.0
            }
        );
    }

    #[test]
    fn a_popup_near_the_bottom_edge_flips_above_the_cursor() {
        let placed = place_popup(
            Point { x: 500.0, y: 100.0 },
            POPUP,
            primary_visible(),
            DESKTOP_HEIGHT,
            GAP,
        );
        assert_eq!(
            placed,
            Point {
                x: 508.0,
                y: 732.0
            }
        );
    }

    #[test]
    fn a_popup_near_the_right_edge_is_pulled_inside_the_screen() {
        let placed = place_popup(
            Point {
                x: 1_900.0,
                y: 500.0,
            },
            POPUP,
            primary_visible(),
            DESKTOP_HEIGHT,
            GAP,
        );
        assert_eq!(
            placed,
            Point {
                x: 1_500.0,
                y: 588.0
            }
        );
    }

    #[test]
    fn a_popup_too_tall_for_the_screen_is_clamped_to_the_visible_top() {
        let tall = Size {
            width: 420.0,
            height: 1_000.0,
        };
        let placed = place_popup(
            Point {
                x: 500.0,
                y: 1_000.0,
            },
            tall,
            primary_visible(),
            DESKTOP_HEIGHT,
            GAP,
        );
        assert_eq!(placed.y, 25.0, "the menu-bar edge, not a negative offset");
    }

    #[test]
    fn the_second_screen_is_used_in_its_own_global_place() {
        let placed = place_popup(
            Point {
                x: 2_000.0,
                y: 200.0,
            },
            POPUP,
            secondary_visible(),
            DESKTOP_HEIGHT,
            GAP,
        );
        assert_eq!(
            placed,
            Point {
                x: 2_008.0,
                y: 632.0
            }
        );
    }

    #[test]
    fn a_point_scale_desktop_keeps_the_top_left_point_unchanged() {
        assert_eq!(
            to_physical(
                Point {
                    x: 508.0,
                    y: 588.0
                },
                1_080.0,
                1_080.0,
                1.0
            ),
            Point {
                x: 508.0,
                y: 588.0
            }
        );
    }

    #[test]
    fn a_retina_desktop_flip_uses_the_pixel_height_tao_uses() {
        // On a 2x primary, the top-left of the desktop is y = 0 physically, but
        // tao flips with 2160, so the conversion has to compensate.
        assert_eq!(
            to_physical(
                Point { x: 0.0, y: 0.0 },
                1_080.0,
                2_160.0,
                2.0
            ),
            Point {
                x: 0.0,
                y: 2_160.0
            }
        );
    }

    #[test]
    fn a_window_on_a_point_scale_secondary_uses_the_window_scale() {
        // Cursor is on the 1x primary, the window is still a 1x window, and the
        // primary is 2x: the backend divides by 1 and flips with 2160.
        assert_eq!(
            to_physical(
                Point {
                    x: 508.0,
                    y: 588.0
                },
                1_080.0,
                2_160.0,
                1.0
            ),
            Point {
                x: 508.0,
                y: 1_668.0
            }
        );
    }
}
