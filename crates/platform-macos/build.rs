/// The capture bridge is Objective-C because Accessibility, NSPasteboard and
/// synthesized key events are Objective-C surfaces. Compiling it here keeps every
/// unsafe platform call inside `platform-macos`
/// (docs/adr/0001-rust-core-with-tauri-shell.md).
fn main() {
    println!("cargo:rerun-if-changed=src/macos/capture.m");
    println!("cargo:rerun-if-changed=src/macos/capture.h");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }

    cc::Build::new()
        .file("src/macos/capture.m")
        .flag("-fobjc-arc")
        .flag("-Wall")
        .compile("translator_capture");

    for framework in [
        "AppKit",
        "ApplicationServices",
        "Carbon",
        "CoreGraphics",
        "Foundation",
    ] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
}
