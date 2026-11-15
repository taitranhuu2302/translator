use std::path::PathBuf;

/// The packaged plist is the single authoring point for the identifier; the
/// running app asks its own bundle, and this compile-time copy is the fallback
/// for executables that have no bundle (the test binaries).
fn bundle_identifier() -> String {
    let plist = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packaging/Info.plist");
    println!("cargo:rerun-if-changed={}", plist.display());

    let xml = std::fs::read_to_string(&plist)
        .unwrap_or_else(|error| panic!("reading {}: {error}", plist.display()));
    plist_string(&xml, "CFBundleIdentifier")
        .unwrap_or_else(|| panic!("{} declares no CFBundleIdentifier", plist.display()))
}

/// The `<string>` following `<key>key</key>`. A plist this small does not
/// justify a dependency; anything unexpected in the file fails the build
/// loudly instead of silently packaging the wrong identity.
fn plist_string(xml: &str, key: &str) -> Option<String> {
    let marker = format!("<key>{key}</key>");
    let rest = &xml[xml.find(&marker)? + marker.len()..];
    let start = rest.find("<string>")? + "<string>".len();
    let end = rest[start..].find("</string>")? + start;
    Some(rest[start..end].trim().to_owned())
}

/// The capture and speech bridges are Objective-C because Accessibility,
/// NSPasteboard, synthesized key events, AVSpeechSynthesizer and NSBundle are
/// Objective-C surfaces. Compiling them here keeps every unsafe platform call
/// inside `platform-macos` (docs/adr/0001-rust-core-with-tauri-shell.md).
fn main() {
    for file in ["capture", "clipboard", "speech", "pointer", "bundle"] {
        println!("cargo:rerun-if-changed=src/macos/{file}.m");
        println!("cargo:rerun-if-changed=src/macos/{file}.h");
    }

    println!("cargo:rustc-env=TRANSLATOR_BUNDLE_ID={}", bundle_identifier());

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }

    cc::Build::new()
        .file("src/macos/capture.m")
        .file("src/macos/clipboard.m")
        .file("src/macos/speech.m")
        .file("src/macos/pointer.m")
        .file("src/macos/bundle.m")
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
