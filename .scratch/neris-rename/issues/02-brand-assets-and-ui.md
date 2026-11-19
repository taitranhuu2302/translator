# 02 — Give the app its mark

Status: done

## What

Build the three brand assets from the downloaded artwork and put the mark where a person sees it: the bundle icon, the menu bar item, and the onboarding and Settings headers.

## Where

- `crates/app/icons/icon.svg`, `icon.png` — white mark on the ocean gradient tile
- `crates/app/icons/tray-template.svg`, `tray-template.png` — black template image
- `crates/app/src/main.rs` — tray uses the template with `icon_as_template(true)`
- `ui/src/components/NerisMark.tsx` — the in-app mark, new file
- `ui/src/Onboarding.tsx`, `ui/src/components/SettingsSheet.tsx` — where the mark is drawn

## Notes

The source SVG's background is a raster JPEG; only the two paths are vector. Every asset here is drawn from the paths alone, so it carries the correct alpha rather than the white the SVG renderer flattens onto.

## Done when

- The packaged bundle shows the icon in Finder and the Dock, with no white corner.
- The menu bar item is legible in both menu bar appearances.
- The onboarding dialog and the Settings header show the mark.
