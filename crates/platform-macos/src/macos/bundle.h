#ifndef TRANSLATOR_BUNDLE_H
#define TRANSLATOR_BUNDLE_H

/*
 * The identifier of the bundle this process is running from, so the Application
 * Support directory and the Keychain service derive from the same identity
 * macOS reads from Info.plist. It is Objective-C because NSBundle is; every
 * platform call stays in this crate
 * (docs/adr/0001-rust-core-with-tauri-shell.md).
 *
 * Returns a newly allocated UTF-8 string owned by the caller (tl_string_free),
 * or NULL when the executable is not inside a bundle — a test binary.
 */
char *tl_bundle_identifier(void);

#endif
