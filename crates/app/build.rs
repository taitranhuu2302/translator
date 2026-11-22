use std::path::PathBuf;

/// The marker `scripts/build.sh` looks for when deciding whether `ui/dist` holds
/// the real frontend. Authored once in `ui/placeholder-marker.txt`; both this
/// writer and that reader take the same value from there, so they cannot drift.
const PLACEHOLDER_MARKER: &str = include_str!("../../ui/placeholder-marker.txt");

fn main() {
    check_icon();

    // `tauri::generate_context!` reads `frontendDist` at compile time, so the path
    // must exist even when someone runs `cargo test` without building the UI first.
    // The placeholder keeps that working; a packaged build replaces it.
    let dist = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ui/dist");
    if !dist.join("index.html").exists() {
        std::fs::create_dir_all(&dist).expect("creating the ui/dist placeholder directory");
        std::fs::write(
            dist.join("index.html"),
            format!(
                "<!doctype html><meta charset=\"utf-8\">\
                 <title>Neris</title>{}\
                 <p>The frontend has not been built. Run `npm run build`.</p>",
                PLACEHOLDER_MARKER.trim()
            ),
        )
        .expect("writing the ui/dist placeholder");
    }

    println!("cargo:rerun-if-changed=../../ui/dist");
    println!("cargo:rerun-if-changed=../../ui/placeholder-marker.txt");
    tauri_build::build()
}

/// Tauri embeds `icon.png`'s rows verbatim and the runtime reads them as 8-bit
/// RGBA. A 16-bit PNG — what an image editor renders by default — supplies twice
/// the bytes the dimensions allow, so the app panics at setup with "invalid
/// icon" instead of failing here. `tauri dev`, `cargo build` and the packaging
/// script all run this, which makes the build the one place the rule is checked.
fn check_icon() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("icons/icon.png");
    let bytes = std::fs::read(&path).expect("reading the app icon");
    const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    assert!(
        bytes.starts_with(&PNG_SIGNATURE),
        "{} is not a PNG",
        path.display()
    );
    // IHDR is the first chunk: signature (8), length (4), type (4), width (4),
    // height (4), then the bit depth and the colour type, one byte each.
    let depth = bytes[24];
    let colour = bytes[25];
    if depth != 8 || colour != 6 {
        panic!(
            "{} is {depth}-bit with PNG colour type {colour}; Tauri needs 8-bit RGBA \
             (colour type 6). Re-encode it: magick icon.png -depth 8 \
             -define png:color-type=6 PNG32:icon.png",
            path.display()
        );
    }
    println!("cargo:rerun-if-changed=icons/icon.png");
}
