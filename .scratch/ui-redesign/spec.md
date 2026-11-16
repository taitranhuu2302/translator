# UI redesign — one Neris design system

Status: ready-for-agent

## Problem Statement

The app's behaviour is settled and heavily tested; its appearance has never been designed. Every surface was built to be readable rather than to be looked at, and the result reads as a developer tool wearing the app's name.

The evidence is concrete:

- **There is no system.** `ui/src/styles.css` carries six ad-hoc variables (`--edge`, `--surface`, `--accent: #2f6fdb`, `--warning`, `--danger`, `--ok`) and seven corner radii — 5, 6, 7, 8, 10, 14 and 999px. Spacing is typed per rule: 8, 10, 11, 12, 14, 18, 20 and 22px. Nothing makes a second card agree with the first.
- **The palette is not the brand.** The app follows the system appearance (`color-scheme: light dark`, `Canvas`/`CanvasText`), so in daylight it is a light-grey form. The ocean palette in `DESIGN.md` exists only on paper, and `--accent` is `#2f6fdb`, not Aurora Blue.
- **Type is the platform default.** `-apple-system` at 13px. `DESIGN.md` specifies Inter and JetBrains Mono with Inter's stylistic alternates; neither face is loaded anywhere.
- **Nothing responds.** No `:focus-visible`, no `transition`, no `@keyframes`, no `prefers-reduced-motion`, no custom scrollbar, and no icon at all — every button is a word. One hover is not merely subtle but absent: `.primary` sets `border-color: transparent` and the only hover rule changes `border-color`.
- **The frame is too small for what it holds.** The main window is 720×480 with no minimum size. Two panes with a 180px minimum, a tab strip and a topbar leave roughly 300px of height for the work itself.
- **Four surfaces are peers in a tab row.** `translate`, `improve`, `history` and `settings` sit in one row as equals, so opening History removes the translation in progress from view, and Improve — the same two-pane shape with a different verb — is presented as a sibling of History rather than as a mode.
- **The popup is a rectangle.** A fixed, opaque 420×220 window scrolls long answers inside a fixed height and leaves a large empty area under short ones.

`DESIGN.md` describes a shadcn/ui token system this codebase has never had, and alongside it an overlay window and a sidebar that do not exist. It is not a description of the app; it is the target the app has not reached.

## Solution

Adopt **Tailwind CSS v4 + shadcn/ui** as the UI foundation, express `DESIGN.md`'s ocean palette as the token layer in HSL exactly as written, make the app genuinely dark-only, and restructure the window shell around a unified toolbar. Five surfaces, one system, one vocabulary.

What changes for the person using it:

1. The app looks like one product. Every card, field, button and message comes from the same scale and the same palette.
2. It is dark, always, in the palette `DESIGN.md` specifies.
3. Translate and Improve are a mode switch inside the same frame, and History and Settings open over the work instead of replacing it.
4. The window is big enough to work in, and the popup is a floating card that fits the answer.

Nothing about *what* the app does changes: no new feature, no new shortcut, no new provider, no changed flow.

## User Stories

1. As someone translating, I want the window to be large enough that a Vietnamese sentence and its translation are both readable without constant scrolling.
2. As someone translating, I want the two panes to be obviously the same frame in two roles, so that I read them as input and output rather than as two unrelated boxes.
3. As someone using Improve, I want it to feel like the same window doing a different job, not a different screen I have to navigate to.
4. As someone checking a past translation, I want History to open beside the work rather than replace it, so that I do not lose what I was reading.
5. As someone changing a setting, I want Settings to open over the window and close back to exactly where I was.
6. As someone using Quick Translate, I want the popup to look like a card floating over my app, not like a small window I have to manage.
7. As someone translating a long sentence in the popup, I want the popup to grow to fit rather than scroll inside a fixed height.
8. As someone translating a short one, I want the popup to be short.
9. As someone working at night, I want the app to be dark without my whole system having to be.
10. As someone reading Vietnamese, I want every diacritic rendered in the app's own face — never a fallback glyph that sits lower or thinner than its neighbours.
11. As someone who moves by keyboard, I want to see where focus is on every control, including inside the panes.
12. As someone using a screen reader, I want the mode switch to be a real tablist and the panels to be real dialogs, exactly as the hand-written markup pretends to be today.
13. As someone who prefers reduced motion, I want the panel and sheet transitions removed rather than merely shortened.
14. As someone scanning a failed translation, I want the failure to carry an icon and a colour that match its cause, so that I can tell a provider failure from a missing Accessibility grant at a glance.
15. As someone waiting for an answer, I want a skeleton in the shape of what is coming rather than a line of text, so that the answer does not make the layout jump.
16. As someone with an empty History, I want a composed empty state rather than a lone sentence.
17. As a developer adding a control, I want the primitive to already exist with the app's tokens applied, so that I do not invent a sixth button style.
18. As a developer editing the palette, I want one file to change, so that every surface moves together.
19. As a developer adding a surface, I want the shell's toolbar and panel slots to be the obvious places to hook in.
20. As a developer opening the popup code, I want the rule that it takes no Radix component stated where I will read it, so that I do not add a dropdown that cannot open.
21. As a developer reviewing a UI change, I want to see the whole app with mock data before it is wired to the backend, so that design review is not a build cycle.
22. As an agent working on the UI, I want a project skill that states the conventions, so that a change made in a later session still looks like this one.

## Implementation Decisions

### A. Foundation

**A1. Tailwind CSS v4 with shadcn/ui.** Tailwind v4 is CSS-first: tokens are declared in `@theme` inside the stylesheet and there is no `tailwind.config.js`. `@tailwindcss/vite` declares Vite `^5.2 || ^6 || ^7 || ^8` as a peer, which covers the repo's Vite 8. shadcn 4.x is installed with `npx shadcn@latest init` run **inside `ui/`** — the npm workspace root has no application to detect.

The `@/*` import alias is added to `ui/tsconfig.json` (`baseUrl` + `paths`) and to `vite.config.ts` (`resolve.alias`), which is what both the CLI and every generated component expect.

**A2. One token layer, named as shadcn names it.** `ui/src/index.css` replaces `ui/src/styles.css` and contains: `@import "tailwindcss"`, the `@theme` block, and a `@layer base` for the handful of rules that are not utilities (`color-scheme`, the shell's height chain, scrollbar styling, the popup's transparent body). The toolbar's drag region is not a CSS rule — see B2.

Every shadcn token is given `DESIGN.md`'s value **in HSL, verbatim**: `--background`, `--foreground`, `--card`, `--popover`, `--primary`, `--secondary`, `--muted`, `--accent`, `--destructive`, `--border`, `--input`, `--ring`, the `--sidebar-*` set, the `--chart-*` set, and `--radius: 0.75rem` with its derived scale. `DESIGN.md`'s conceptual `--neris-*` names do **not** survive as an alias layer: the palette is written straight onto the semantic tokens, so there is exactly one name per colour and no second vocabulary to drift. The comment above `:root` names `DESIGN.md` as the source.

`shadcn init` writes its defaults in `oklch`; those are replaced wholesale, not merged. Any token shadcn emits that `DESIGN.md` does not name (for example `--muted-foreground` variants) is derived from the nearest named colour rather than left at shadcn's default.

**A3. Dark-only, enforced rather than implied.** `color-scheme: dark` in the base layer; the main window configured with `theme: "Dark"`; `<html class="dark">` set permanently. The `.dark` class stays even though it is the only theme, because several shadcn components read `.dark` directly rather than reading a variable, and removing it breaks them silently. Every `Canvas`/`CanvasText`/`GrayText` usage in the old stylesheet is deleted, not ported.

**A4. Inter and JetBrains Mono, bundled, with the Vietnamese subset.** Fonts are installed as `@fontsource-variable/inter` and `@fontsource-variable/jetbrains-mono` and imported through the single stylesheet. Their stylesheets carry every subset the packages ship, `latin` and `vietnamese` among them, so the built bundle also contains `cyrillic`, `greek` and `latin-ext` faces that this app will never select; pruning them would mean copying fontsource's `unicode-range` values into our own `@font-face` rules, which is more magic to maintain than the bytes are worth in a local app. Inter's `"cv02", "cv03", "cv04", "cv11"` feature settings are applied on the body. **Vietnamese diacritics are a hard acceptance criterion**: a rendered check string containing ắ ệ ỏ ữ ợ ằ ẩ ỹ must show no glyph falling back to another face — and it does, checked in the harness on both surfaces.

**A5. `styles.css` is deleted.** It is not kept as a base or a parallel system. The rules worth keeping are rewritten in the `@layer base`, in the new vocabulary, and the rest — seven radii, six ad-hoc variables, per-rule spacing — are gone by construction. Two style systems in one app is the exact condition that produced the current state.

**A6. A dev-only preview harness.** `ui/preview.html` plus `ui/src/preview/` render every surface with mock data and every state (loading, empty, error, long content). It is reachable only from the Vite dev server: it is not added to `build.rollupOptions.input`, so the production bundle never contains mock data or the harness. The harness grows with each surface ticket below, and the design is approved in the harness before the app's own components are touched.

### B. Window shell

**B1. The window is 960×640 with a minimum of 680×460.** The remembered frame written by `crates/app/src/main.rs` still wins on launch, so existing users are not resized against their will. The pane minimums stay at 180px, which two panes still satisfy at the new minimum width.

**B2. A unified toolbar.** `titleBarStyle: "Overlay"` with `hiddenTitle`, so the traffic lights float over the app's own toolbar row and the separate title bar disappears — `DESIGN.md`'s "unified toolbar (no separate title bar)". The toolbar reserves the traffic-light inset on the left and is a drag region.

The drag region is Tauri's, not CSS: the header carries `data-tauri-drag-region`, which the shell acts on in WKWebView and which only applies to the element that has it, so interactive children keep their own clicks without a `no-drag` counterpart. It needs `core:window:allow-start-dragging` in `crates/app/capabilities/default.json`, because `core:default` does not include it. Electron's `-webkit-app-region: drag` does nothing here; an early draft used it and the window would not have dragged.

**B3. Translate and Improve are a mode, not a destination.** They become a segmented control built from shadcn `Tabs` in the toolbar, keeping `role="tablist"` and `aria-selected`, which the current markup already sets by hand. History and Settings move to the toolbar's trailing side as icon buttons.

**B4. A footer states the two keys.** One line: the Translate shortcut and the Replace shortcut. This replaces nothing that exists today except the shortcut hint currently in the topbar.

The provider does **not** go in the footer. The plan had it there, but nothing in the app names an "active provider": `Settings` holds per-provider keys, and the provider is decided per request and returned with the answer. The badge therefore lives in the result pane's header (`via Azure`), where the name is a fact about the answer on screen rather than a guess about the configuration — see C1 and ticket 04.

### C. Surfaces

**C1. The two panes.** `1fr 1fr`, 16–24px gutters, each pane a bordered card with a small uppercase label, and the result pane's actions in its own header rather than at the bottom. The primary-button hover is fixed as part of this: `.primary` no longer sets a transparent border that the only hover rule then fails to change.

**C2. History is a push panel.** shadcn `Sheet` sliding from the right, with the search field, the list and the empty state. It does not replace the panes, so the work stays visible beside it.

**C3. Settings is a sheet.** The five existing sections (Language Pair, Startup, Shortcuts, Providers, Reset) keep their content and their order; only the frame and the controls change. Provider key rows keep their current behaviour and wording, which tickets 05 and 06 of the architecture effort deliberately consolidated.

**C4. States have a vocabulary.** `Skeleton` in the shape of the content being loaded (panes, history list), a spinner inside a button during a request, `Alert` with an icon and a colour per cause for failures, and one `EmptyState` primitive for empty panes, empty History and empty results. The existing `notice.ts` model remains the single source of wording; `NoticeView` renders it through `Alert` instead of hand-rolled markup.

**C5. Onboarding becomes a `Dialog`.** Same content and same one-time behaviour, expressed with the primitive so it inherits focus trapping and escape handling instead of relying on a backdrop div.

### D. Quick Translate popup

**D1. A transparent card.** The popup window gains `transparent: true`, and the page renders a rounded card with the design system's shadow on a transparent body. It keeps its width of 420, its position near the cursor, `alwaysOnTop`, `skipTaskbar` and `decorations: false`.

**D2. Height follows content.** The window height is computed from the rendered card, clamped between 120 and 320 logical pixels, and the panel is re-placed after resizing so it does not drift away from the cursor or off the screen edge. This replaces the unconditional 420×220 enforced on every show.

**D3. The popup uses no Radix component.** It is composed from native elements styled with Tailwind. The stated reason is not that the popup is deaf — it is not; ADR-0005 establishes that it takes keys — but that a portal-rendered dropdown inside a 420px window is clipped and that the popup has no form that needs one. This rule is written down in the project skill so it is not re-litigated by accident.

**D4. ADR-0005 is not disturbed.** The popup remains a non-activating `NSPanel` that can become key, keeps its style mask, level and collection behaviour, and keeps the first-responder hand-off into the web view. Transparency and auto-height are window properties and content changes; they must not alter any of that. Any Tauri upgrade still requires re-running ADR-0005's spike checks.

### E. Records and process

**E1. The work is cut into tickets**, one per independently shippable change, under `.scratch/ui-redesign/issues/`, with manual test cases under `checks/` for anything that needs a human eye — which here is most of it, since the app is never launched by an agent.

**E2. A project skill records the conventions**: how to add a shadcn component in this repo, the dark-only rule, the popup's no-Radix rule, the font subset requirement, the radius scale, and the preview harness. It lives at `.claude/skills/neris-ui/SKILL.md`.

**E3. The existing test suite is untouched by this work.** All thirteen UI test files are pure-logic modules; none assert markup, and there is no testing-library. Restyling therefore cannot break them, and any test change in this effort is an addition for a new helper, never a repair of a design-caused failure.

## Testing Decisions

**What is testable here.** Layout and appearance are not: they are checked by eye in the preview harness and then in the signed bundle. What *is* testable is the small amount of logic the redesign introduces, and it is tested the way the rest of the UI already is — a pure module driven by explicit inputs.

- **Popup height clamping** — a pure function from content height to window height, tested at the boundaries (below 120, above 320, inside) and with a known screen edge so the re-placement rule is exercised.
- **Mode/surface selection** — any new mapping from a toolbar action to a surface is a pure function and gets the same treatment as `settingsView` and `historyView`.
- **Token sanity — not tested.** CSS variable values are not asserted in code; they are checked in the harness. A test that duplicates a hex value only guarantees that both copies were edited together.

**Manual cases.** Everything visual is handed over as a check file: the app is built and signed by the agent but never launched, and the person runs the bundle and walks the table.

## Out of Scope

- Any behaviour change: no new feature, shortcut, provider, settings key or migration of stored data.
- The Rust crates beyond three window properties (size and minimum, title bar style, popup transparency) and the popup's sizing call.
- Localisation. The UI stays in English in this effort; the copy, `GLOSSARY.md` and the tests stay in their present language.
- `DESIGN.md`'s sidebar, overlay-window and chart sections. They describe surfaces this app does not have; the palette and type sections are adopted, the rest is not built.
- A light theme, a theme toggle, or following the system appearance.
- Rewriting `notice.ts`, `settingsWrite.ts`, `historyView.ts`, `popupState.ts`, `answerToken.ts` or any other logic module consolidated by the architecture effort.

## Further Notes

**Three assumptions were corrected while planning, and the corrections matter to the tickets.**

1. The popup is **not** keyboard-deaf. `focus: false` means it does not steal focus when shown; ADR-0005 has it become key and make the web view first responder, which is how Enter-to-Replace works. D3 stands for a different reason than originally given.
2. There are **no DOM tests** in this repo — thirteen `.test.ts` files, all pure logic, no `.test.tsx`. Markup changes cannot make the suite red, so the suite is evidence about nothing visual.
3. The shadcn **MCP server is already configured** in `.mcp.json`. What was missing is `components.json` in `ui/`; without it the CLI and the MCP tools have nothing to act on.

**Why the earlier "no Tailwind" decision was reversed.** The first pass chose hand-written CSS with shadcn-shaped token names, on the grounds that a seven-component app does not need a framework. The user's explicit direction is to use shadcn itself, so that later additions inherit a real component system rather than a home-made approximation. That is recorded as ADR-0006 rather than left as a silent divergence, because the next reader will otherwise find a stylesheet that looks like shadcn and a `package.json` that explains why.

**The deletion test.** This effort is worth doing only if the old stylesheet disappears rather than being wrapped. If `styles.css` survives alongside `index.css`, stop: the second system will drift, and the drift is the problem being solved.

**Corrections found while building.** Each was a claim above that the code contradicted, and each is recorded here so the spec and the app do not disagree:

1. **The toolbar's drag region is not CSS** (B2). The planned `-webkit-app-region: drag` is an Electron property and is inert in WKWebView; the first build would not have dragged. The header now carries Tauri's `data-tauri-drag-region` and the capability `core:window:allow-start-dragging`, which `core:default` does not include.
2. **There is no active provider to print** (B4). The footer states the two shortcuts only; the provider appears as a badge on the answer, where it is known.
3. **Fonts ship every subset** (A4). Pruning to `latin` + `vietnamese` would mean hand-copying `unicode-range`s; the extra faces are dead weight in a local app, not a defect. Vietnamese diacritics were checked by eye in the harness.
4. **The oklch purge is not literally total** (A2). Generated `Button` carries one `hover:bg-[color-mix(in_oklch, …)]`. It is a hover mix of two already-hsl tokens, not a second palette, so it was left rather than hand-rewritten.
5. **A stored frame is not always honoured** (B1). `restore_frame` loads the remembered size, but macOS clamps a window up to the configured minimum, so a user whose old frame was smaller than 680×460 now gets the minimum. Everything at or above the new floor restores untouched.
6. **The dialog had no elevation.** Generated `DialogContent` shipped a 1px ring and nothing else; it was given `shadow-lg` and the cards' radius to match the other floating surfaces. The change lives in the generated component, which is why it is written down in ticket 07.
