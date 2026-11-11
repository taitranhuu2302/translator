//! `core` is the cross-platform promise: no AppKit, no Tauri, no macOS crate.
//! This test keeps that boundary honest as later tickets add code.

use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN_DEPENDENCIES: &[&str] = &[
    "tauri",
    "objc2",
    "cocoa",
    "core-foundation",
    "core-graphics",
    "security-framework",
    "dispatch2",
    "block2",
    "translator-platform-macos",
];

const FORBIDDEN_IMPORTS: &[&str] = &[
    "objc2",
    "cocoa",
    "app_kit",
    "appkit",
    "core_foundation",
    "security_framework",
    "tauri::",
];

#[test]
fn core_dependencies_are_platform_free() {
    let manifest = fs::read_to_string(manifest_dir().join("Cargo.toml")).expect("core manifest");
    let dependencies = dependency_tables(&manifest).to_lowercase();

    for name in FORBIDDEN_DEPENDENCIES {
        assert!(
            !dependencies.contains(name),
            "translator-core must not depend on {name}"
        );
    }
}

#[test]
fn dependency_tables_cover_every_cargo_form() {
    let manifest = r#"
[package]
name = "translator-core"

[dependencies]
serde = "1"

[dependencies.objc2]
version = "0.5"

[dev-dependencies]
tempfile = "3"

[target.'cfg(target_os = "macos")'.dependencies]
cocoa = "0.26"
"#;

    let tables = dependency_tables(manifest).to_lowercase();

    for name in ["serde", "objc2", "tempfile", "cocoa"] {
        assert!(tables.contains(name), "missed {name}");
    }
    assert!(!tables.contains("[package]"));
}

#[test]
fn core_sources_do_not_import_platform_apis() {
    for path in rust_sources(&manifest_dir().join("src")) {
        let source = fs::read_to_string(&path).expect("core source");
        for (index, line) in source.lines().enumerate() {
            let code = line.trim().to_lowercase();
            if !(code.starts_with("use ") || code.starts_with("extern crate")) {
                continue;
            }
            for needle in FORBIDDEN_IMPORTS {
                assert!(
                    !code.contains(needle),
                    "{}:{} imports platform API {needle}",
                    path.display(),
                    index + 1
                );
            }
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Every table Cargo accepts for a dependency: `[dependencies]`,
/// `[dev-dependencies]`, `[build-dependencies]`, the table form
/// `[dependencies.objc2]`, and target-specific tables such as
/// `[target.'cfg(target_os = "macos")'.dependencies]`. The header line is kept
/// because the table form names its crate in the header itself.
fn dependency_tables(manifest: &str) -> String {
    let mut body = String::new();
    let mut inside = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            inside = trimmed.to_lowercase().contains("dependencies");
            if inside {
                body.push_str(trimmed);
                body.push('\n');
            }
            continue;
        }
        if inside {
            body.push_str(trimmed);
            body.push('\n');
        }
    }
    body
}

fn rust_sources(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let entries = fs::read_dir(dir).unwrap_or_else(|_| panic!("reading {}", dir.display()));
    for entry in entries {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            files.extend(rust_sources(&path));
        } else if path.to_string_lossy().ends_with(".rs") {
            files.push(path);
        }
    }
    files
}
