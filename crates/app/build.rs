use std::path::PathBuf;

/// Marker written into the placeholder so `scripts/build.sh` can refuse to package
/// a bundle that was compiled without the real frontend.
const PLACEHOLDER_MARKER: &str = "<!-- translator-ui-placeholder -->";

fn main() {
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
                 <title>Translator</title>{PLACEHOLDER_MARKER}\
                 <p>The frontend has not been built. Run `npm --prefix ui run build`.</p>"
            ),
        )
        .expect("writing the ui/dist placeholder");
    }

    println!("cargo:rerun-if-changed=../../ui/dist");
    tauri_build::build()
}
