# Swift + SwiftUI instead of Tauri

The app's value is almost entirely in native macOS behaviour: a global hotkey, reading the selection in whatever app has focus, replacing it, a popup that must not steal focus, and speech. Four of those five are direct AppKit/AVFoundation calls; the fifth is a small text UI.

We build a Swift 6 + SwiftUI menu-bar app (`LSUIElement`, no Dock icon) with AppKit where SwiftUI falls short.

The deciding constraint is the popup. It must be a non-activating `NSPanel`, or it becomes the key window, the target app deactivates, and the synthesized ⌘V lands in our own window. In Swift that is a style mask; in Tauri it means `tauri-nspanel` plus hand-patching `canBecomeKeyWindow` through Obj-C FFI, because that plugin hardcodes `YES`. Startup speed reinforces it: 60–90 ms measured for a native accessory app versus ~0.8–1.2 s for Tauri on this machine.

**Consequences:** macOS only — a Windows port would need a rewrite of the capture layer and a different UI stack. There is no web UI tooling, so the design language is expressed in SwiftUI rather than CSS. Auto-update, if ever needed, means adding Sparkle.
