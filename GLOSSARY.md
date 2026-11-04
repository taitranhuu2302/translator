# Translator

A macOS desktop translator that works on text selected in any app, and a window for translating and improving sentences.

## Language

**Selection**:
The text the user has highlighted in another app (or in our own window) when a shortcut fires.
_Avoid_: Highlighted text, marked text

**Language Pair**:
The two languages the app translates between, with one marked as the current target. Text already in the target language is translated into the other member of the pair.
_Avoid_: Source language, direction

**Target**:
The language the user has configured as the output language. The other member of the Language Pair is the implicit default for input.

**Replace**:
Overwriting the Selection in the app that owns it with its translation.
_Avoid_: Swap, substitute, paste over

**Improve**:
Rewriting a sentence the user wrote so it is correct, and offering a better version.
_Avoid_: Correct, fix, polish

**Quick Translate**:
The shortcut flow: act on the Selection immediately, show a small popup near the cursor, and let the user Replace with one keypress.
_Avoid_: Inline translate, popup translate

**Translate Window**:
The app's own two-pane window: text goes in on one side, the translation or improvement comes out on the other.
_Avoid_: Main window, dashboard

**Read Aloud**:
Speaking the Selection with the system speech voice. It never speaks the translation.
_Avoid_: TTS, speak, pronunciation
