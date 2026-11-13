#ifndef TRANSLATOR_SPEECH_H
#define TRANSLATOR_SPEECH_H

#include <stdint.h>

/*
 * Speaking the Selection with the system voice. AVFoundation is an Objective-C
 * surface, so it lives here rather than in Rust
 * (docs/adr/0001-rust-core-with-tauri-shell.md).
 */

#define TL_SPEECH_OK 0
#define TL_SPEECH_NO_VOICE 1
#define TL_SPEECH_UNAVAILABLE 2

/* Starts speaking `text` in `language` (a BCP-47-style code such as "en" or
 * "vi") and returns at once. A voice already speaking is stopped. Answers
 * TL_SPEECH_NO_VOICE when the system has no installed voice for the language. */
int32_t tl_speak(const char *text, const char *language);

#endif
