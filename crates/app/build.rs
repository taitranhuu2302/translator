use std::path::PathBuf;

/// The marker `scripts/build.sh` looks for when deciding whether `ui/dist` holds
/// the real frontend. Authored once in `ui/placeholder-marker.txt`; both this
/// writer and that reader take the same value from there, so they cannot drift.
const PLACEHOLDER_MARKER: &str = include_str!("../../ui/placeholder-marker.txt");

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
