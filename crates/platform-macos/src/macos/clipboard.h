#ifndef TRANSLATOR_CLIPBOARD_H
#define TRANSLATOR_CLIPBOARD_H

#include <stdbool.h>

/**
 * Replace the general pasteboard with `text` as plain text.
 * Answers whether AppKit accepted the write.
 */
bool tl_write_clipboard(const char *text);

#endif
