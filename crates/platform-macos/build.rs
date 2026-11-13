/// The capture and speech bridges are Objective-C because Accessibility,
/// NSPasteboard, synthesized key events and AVSpeechSynthesizer are Objective-C
/// surfaces. Compiling them here keeps every unsafe platform call inside
/// `platform-macos` (docs/adr/0001-rust-core-with-tauri-shell.md).
fn main() {
    for file in ["capture", "clipboard", "speech", "pointer"] {
        println!("cargo:rerun-if-changed=src/macos/{file}.m");
        println!("cargo:rerun-if-changed=src/macos/{file}.h");
    }

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }

    cc::Build::new()
        .file("src/macos/capture.m")
        .file("src/macos/clipboard.m")
        .file("src/macos/speech.m")
        .file("src/macos/pointer.m")
        .flag("-fobjc-arc")
        .flag("-fblocks")
        .flag("-Wall")
        .compile("translator_capture");

    for framework in [
        "AVFoundation",
        "AppKit",
        "ApplicationServices",
        "Carbon",
        "CoreGraphics",
        "Foundation",
    ] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
}
