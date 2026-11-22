# Neris

A macOS desktop translator that works on text selected in any app, and a window for translating and improving sentences.

## Language

**Selection**:
The text the user has highlighted in another app (or in our own window) when a shortcut fires.
_Avoid_: Highlighted text, marked text

**Language Pair**:
The two languages the app translates between, with one marked as the current target. Text already in the target language is translated into the other member of the pair.
_Avoid_: Source language, direction

**Detection**:
The app's own reading of which language a piece of text is written in, made before any request so the Direction can be chosen. It is not the language a provider reports having detected.
_Avoid_: Auto-detect, language identification, LID

**Target**:
The language the user has configured as the output language. The other member of the Language Pair is the implicit default for input.

**Direction**:
The source and target one translation runs between, resolved from the text being translated. It is not a setting, and it may differ from the Language Pair's Target.
_Avoid_: Mode, way, arrow

**Replace**:
Overwriting the Selection in the app that owns it with its translation.
_Avoid_: Swap, substitute, paste over

**Replace HUD**:
The small capsule beside the cursor that names the step a Replace is on. It appears only when the Replace outlasts a short grace period, never takes the keyboard, and never explains a failure.
_Avoid_: Toast, tooltip, progress popup

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
