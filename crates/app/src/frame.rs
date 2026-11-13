//! The Translate Window's remembered frame.
//!
//! The frame is physical pixels in the global display coordinates Tauri reports,
//! so it means the same thing on a Retina and a non-Retina display. It is a
//! window preference, not a setting: it lives in its own `window-frame.json`
//! beside the settings document and is never part of `Settings`.
//!
//! Restoring is only safe when the saved rectangle still touches a connected
//! display. A monitor that was unplugged would otherwise put the window where no
//! one can reach it, so the caller drops such a frame and falls back to the
//! default position.

use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A window frame in physical pixels. `x`/`y` are the top-left corner in the
/// global display space, which can be negative for a display left of the main one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frame {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// One monitor's usable frame, in the same physical coordinates as [`Frame`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Frame {
    /// Whether any part of the frame is on a connected display. A zero-sized
    /// frame is never usable; neither is one entirely on a monitor that is gone.
    pub fn is_on_any_screen(&self, screens: &[ScreenRect]) -> bool {
        self.width > 0 && self.height > 0 && screens.iter().any(|screen| self.intersects(screen))
    }

    fn intersects(&self, screen: &ScreenRect) -> bool {
        // i64 keeps a large coordinate plus its size from overflowing i32.
        let left = i64::from(self.x);
        let right = left + i64::from(self.width);
        let top = i64::from(self.y);
        let bottom = top + i64::from(self.height);
        let screen_left = i64::from(screen.x);
        let screen_right = screen_left + i64::from(screen.width);
        let screen_top = i64::from(screen.y);
        let screen_bottom = screen_top + i64::from(screen.height);

        left < screen_right && screen_left < right && top < screen_bottom && screen_top < bottom
    }
}

/// The remembered frame on disk. Loading is forgiving: a missing, unreadable or
/// corrupt file means "no remembered frame", never an error the app has to tell
/// the user about.
pub struct FrameStore {
    path: PathBuf,
}

impl FrameStore {
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn load(&self) -> Option<Frame> {
        let bytes = fs::read(&self.path).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    pub fn save(&self, frame: Frame) -> std::io::Result<()> {
        write_atomically(&self.path, &frame)
    }
}

/// Same reasoning as the settings file: a half-written frame is worse than none.
fn write_atomically(path: &Path, frame: &Frame) -> std::io::Result<()> {
    let directory = path.parent().ok_or_else(|| {
        std::io::Error::new(ErrorKind::InvalidInput, "the frame path has no directory")
    })?;
    fs::create_dir_all(directory)?;

    let json = serde_json::to_vec_pretty(frame)
        .map_err(|error| std::io::Error::new(ErrorKind::InvalidData, error))?;
    let staging = path.with_extension("json.tmp");
    {
        let mut file = fs::File::create(&staging)?;
        file.write_all(&json)?;
        file.sync_all()?;
    }
    fs::rename(staging, path)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join(format!("translator-frame-{nanos}"))
            .join(name)
    }

    fn screen(x: i32, y: i32, width: u32, height: u32) -> ScreenRect {
        ScreenRect {
            x,
            y,
            width,
            height,
        }
    }

    fn frame(x: i32, y: i32, width: u32, height: u32) -> Frame {
        Frame {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn a_frame_inside_a_connected_display_is_usable() {
        let screens = [screen(0, 0, 2560, 1440)];

        assert!(frame(120, 80, 720, 480).is_on_any_screen(&screens));
    }

    #[test]
    fn a_frame_on_a_display_left_of_the_main_one_is_usable() {
        // A second display placed to the left has a negative origin.
        let screens = [screen(0, 0, 2560, 1440), screen(-1920, 0, 1920, 1080)];

        assert!(frame(-1800, 100, 720, 480).is_on_any_screen(&screens));
    }

    #[test]
    fn a_frame_on_a_display_that_is_gone_is_dropped() {
        // The saved frame is where a second display used to be.
        let screens = [screen(0, 0, 2560, 1440)];

        assert!(!frame(-1800, 100, 720, 480).is_on_any_screen(&screens));
    }

    #[test]
    fn a_partly_visible_frame_is_still_usable() {
        let screens = [screen(0, 0, 1440, 900)];

        // Most of the window hangs off the bottom-right, but its title bar does not.
        assert!(frame(1200, 700, 720, 480).is_on_any_screen(&screens));
    }

    #[test]
    fn a_frame_that_only_touches_an_edge_is_not_usable() {
        // A title bar at y = 900 sits exactly on the bottom edge, so no pixel of
        // the window is on the display.
        let screens = [screen(0, 0, 1440, 900)];

        assert!(!frame(0, 900, 720, 480).is_on_any_screen(&screens));
    }

    #[test]
    fn a_zero_sized_frame_is_not_usable() {
        let screens = [screen(0, 0, 1440, 900)];

        assert!(!frame(100, 100, 0, 0).is_on_any_screen(&screens));
        assert!(!frame(100, 100, 720, 0).is_on_any_screen(&screens));
    }

    #[test]
    fn with_no_displays_there_is_nothing_to_restore() {
        assert!(!frame(100, 100, 720, 480).is_on_any_screen(&[]));
    }

    #[test]
    fn a_frame_survives_a_round_trip() {
        let path = temp_path("window-frame.json");
        let store = FrameStore::at(&path);
        let saved = frame(-1800, 100, 720, 480);

        store.save(saved).expect("save frame");

        assert_eq!(store.load(), Some(saved));
        assert!(path.exists());
    }

    #[test]
    fn a_missing_or_corrupt_frame_loads_nothing() {
        let path = temp_path("window-frame.json");
        let store = FrameStore::at(&path);
        assert_eq!(store.load(), None);

        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"not json").unwrap();
        assert_eq!(store.load(), None);
    }
}
