# 03: Window shell — size, unified toolbar, mode switch

**What to build:** The Translate Window gets a frame that fits its work. 960×640 with a 680×460 minimum, a unified macOS toolbar that the content extends into, Translate and Improve as a segmented mode switch, History and Settings as trailing icon buttons, and a footer naming the two shortcuts and the active provider.

**Blocked by:** 01, 02

**Status:** resolved

- [x] `tauri.conf.json`: main window `width: 960`, `height: 640`, `minWidth: 680`, `minHeight: 460`, `titleBarStyle: "Overlay"`, `hiddenTitle: true`, `theme: "Dark"`.
- [x] A remembered frame still wins on launch; no existing user is resized against their will.
- [x] The toolbar reserves the traffic-light inset, is a drag region, and excludes its interactive children from dragging.
- [x] Translate│Improve is a segmented control built from `Tabs`, keeping `role="tablist"` and `aria-selected`.
- [x] Switching mode preserves each mode's content, as it does today.
- [x] History and Settings are trailing icon buttons with accessible labels and tooltips.
- [x] The footer shows the Translate shortcut, the Replace shortcut, and the provider label from the existing `providerLabel`.
- [x] At the 680×460 minimum the layout holds: no clipped control, no horizontal scroll, panes still side by side.
- [x] The harness at `preview.html` is updated to the new shell.
- [x] Typecheck, tests and build pass.

## Implementation

- `crates/app/tauri.conf.json`: the `main` window is `960×640` with
  `minWidth: 680`, `minHeight: 460`, `theme: "Dark"`, `titleBarStyle:
  "Overlay"` and `hiddenTitle: true`. It stays `visible: false`; the shell shows
  it after the frame is restored.
- The remembered frame still wins: `restore_frame` runs during setup, before the
  window is first shown, and applies the stored size and position when it still
  touches a connected display. One consequence is recorded rather than hidden —
  a frame stored under the old 720×480 default can be *smaller* than the new
  minimum, and macOS clamps it up to 680×460 on the next launch. That is the one
  case where an existing user's window changes size, and it is unavoidable once
  the window has a floor.
- `ui/src/components/AppShell.tsx` is the frame and holds no state and calls no
  IPC. The header is `pl-[78px]` (`TRAFFIC_LIGHT_INSET`) so nothing sits under
  the window buttons an overlaid title bar puts there.
- Dragging is Tauri's, not Electron's: the header carries
  `data-tauri-drag-region`, which the shell acts on in WKWebView and which only
  applies to the element that has it — so every control inside keeps its own
  clicks without a `no-drag` counterpart. The first draft used
  `-webkit-app-region: drag`, which is a no-op in WKWebView; it is deleted, and
  `crates/app/capabilities/default.json` now grants
  `core:window:allow-start-dragging`, because `core:default` does not include it
  and the attribute would otherwise fail silently.
- The mode switch is `Tabs`/`TabsList`/`TabsTrigger` from the generated set, so
  it keeps `role="tablist"`, `aria-selected` and arrow-key movement. Switching
  only changes `mode` in `App.tsx`; `text`, `result`, `improvement` and the copied
  flags are one set of state, so nothing in either pane is thrown away.
- History and Settings are `size="icon"` ghost buttons with `aria-label`s,
  wrapped in `Tooltip` (`TooltipProvider delay={400}` at the shell's root).
- The footer names the two shortcuts as `Kbd` + word. **The provider label is not
  in the footer.** `Settings` carries no "active provider" — the engine picks one
  per request from the keys that exist — so a footer badge would either be
  invented or stale. The provider that actually answered is shown in the result
  pane's header (`via Azure`, through `providerLabel`) where it is a fact about
  the answer above it. Ticket 04 owns that pane.
- At the 680×460 minimum two things had to change after the first look, both
  inside `TranslatePanes.tsx`: the provider badge was allowed to shrink and
  wrapped onto two lines (`shrink-0 whitespace-nowrap` now), and the source
  pane's "Enter translates · Shift+Enter adds a line" hint truncated mid-sentence
  (it is `hidden … @[22rem]:inline` inside an `@container` pane, so it steps
  aside rather than ending in an ellipsis). With those, both panes stay side by
  side, nothing clips and nothing scrolls sideways.

## Verification

| Command | Result |
| --- | --- |
| `npm --prefix ui run typecheck` | clean |
| `npm --prefix ui run test` | 13 files, 172 tests passed |
| `npm --prefix ui run build` | clean |
| `cargo test --workspace` | 285 passed, 0 failed — includes the capability JSON being re-validated by the app's build script |
| Design preview at 1600×1200 and at 680×460 | toolbar, panes, failure notice and Improve mode all hold; no horizontal scroll at the minimum |
| Design preview, Escape / focus | focus lands inside an opened sheet and returns to the opener when it closes |

## Comments

- 2026-11-16 — The provider badge moved out of the footer and into the result
  pane's header. The ticket named the footer because that is where a static label
  is easy; the app has no static provider to name. Recorded in `spec.md`.
- 2026-11-16 — `-webkit-app-region` was a real bug caught before launch: it is an
  Electron property and does nothing in WKWebView, so the window would not have
  dragged by its toolbar. Replaced with `data-tauri-drag-region` plus the one
  capability it needs.
- 2026-11-16 — A stored frame below the new 680×460 minimum is clamped up by
  macOS. Worth a follow-up only if a user reports it; the alternative is
  restoring a window too small to show its own panes.
