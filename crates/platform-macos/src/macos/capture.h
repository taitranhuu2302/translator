#ifndef TRANSLATOR_CAPTURE_H
#define TRANSLATOR_CAPTURE_H

#include <stdbool.h>
#include <stdint.h>

/*
 * Reading the Selection, the Accessibility permission, and the settings pane it
 * is granted in. The capture ladder lives here rather than in Rust because the
 * Accessibility API, NSPasteboard and synthesized key events are all Objective-C
 * surfaces; docs/adr/0001-rust-core-with-tauri-shell.md keeps them in this crate.
 */

#define TL_CAPTURE_OK 0
#define TL_CAPTURE_NO_ACCESSIBILITY 1
#define TL_CAPTURE_SECURE_INPUT 2
#define TL_CAPTURE_NOTHING_SELECTED 3
#define TL_CAPTURE_UNAVAILABLE 4

/* Returns a newly allocated UTF-8 string owned by the caller (tl_string_free), or
 * NULL with *status set to one of the constants above. */
char *tl_read_selection(int32_t *status);

bool tl_accessibility_granted(void);

/* Ask the system to prompt for Accessibility; answers whether it is granted now. */
bool tl_request_accessibility(void);

/* Open System Settings at Privacy & Security → Accessibility. */
bool tl_open_accessibility_settings(void);

void tl_string_free(char *text);

#endif
