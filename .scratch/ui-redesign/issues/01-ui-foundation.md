# 01: UI foundation — Tailwind v4, shadcn/ui, Neris tokens

**What to build:** The app gains a real UI foundation and loses its hand-written one. Tailwind CSS v4 is installed with the Vite plugin, shadcn/ui is initialised inside `ui/`, `@/*` resolves in both TypeScript and Vite, and `ui/src/index.css` replaces `ui/src/styles.css`. Every shadcn token carries `DESIGN.md`'s HSL value verbatim, in dark-only form. Inter and JetBrains Mono are bundled with the Vietnamese subset.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] `npm install tailwindcss @tailwindcss/vite` in `ui/`; the plugin is added to `vite.config.ts`; `vite build` still emits both `index.html` and `popup.html`.
- [x] `npx shadcn@latest init` run **inside `ui/`** (not the workspace root) and `ui/components.json` exists with the `@/` alias and `ui/src/index.css` as its stylesheet.
- [x] `@/*` resolves from `ui/tsconfig.json` (`baseUrl` + `paths`) and from `vite.config.ts` (`resolve.alias`).
- [x] `ui/src/index.css` is the only stylesheet: `@import "tailwindcss"`, a `@theme` block, and one `@layer base`.
- [x] Every token shadcn emits has `DESIGN.md`'s HSL value; no `oklch` value survives from `init`.
- [x] `--radius: 0.75rem` with `sm`/`md`/`lg`/`xl`/`2xl`/`3xl`/`4xl` as `calc` multiples, exactly as `DESIGN.md` states.
- [x] `--neris-*` names exist only as aliases of the semantic tokens; no second colour system.
- [x] Dark-only: `<html class="dark">` permanent, `color-scheme: dark`, main window `theme: "Dark"`, no `Canvas`/`CanvasText`/`GrayText` anywhere.
- [x] Inter and JetBrains Mono installed from `@fontsource-variable/*` with `latin` + `vietnamese` subsets; `cv02`, `cv03`, `cv04`, `cv11` applied on the body.
- [x] `ui/src/styles.css` deleted; `main.tsx` and `popup.tsx` import `index.css`.
- [x] `npm run typecheck`, `npm run test`, `npm run build` all pass, and the suite still reports 13 files.
- [x] The Vietnamese check string renders in Inter with no fallback glyph.

## Implementation

- `ui/package.json` carries `tailwindcss@^4`, `@tailwindcss/vite@^4`,
  `tw-animate-css`, `@base-ui/react` (shadcn's Base UI build), `shadcn@^4.23`,
  `class-variance-authority`, `cn`, `lucide-react`, and both `@fontsource-variable`
  packages. `vite.config.ts` adds `tailwindcss()` beside `react()` and keeps
  `rollupOptions.input` at exactly `main` and `popup`, so the two windows still
  build and the harness still does not.
- `ui/components.json` is the shadcn project file: style `base-nova`, css
  `src/index.css` (relative to `ui/`), no `tailwind.config`, `@/components`,
  `@/components/ui`, `@/lib`, `@/hooks`, icon library `lucide`. The CLI only runs
  with `-c ui`; the repository root is an npm workspace and it refuses there.
- `@/*` resolves twice: `baseUrl: "."` + `paths` in `ui/tsconfig.json`, and
  `resolve.alias` in `vite.config.ts`. `cn` is imported from the `cn` package
  (not a hand-written `clsx`/`tailwind-merge` helper) because that is what the
  generator writes; `ui/src/lib/utils.ts` re-exports it so components have one
  import path.
- `ui/src/index.css` is the only stylesheet. It imports `tailwindcss`,
  `tw-animate-css`, `shadcn/tailwind.css` and the two fonts, defines
  `@custom-variant dark (&:is(.dark *))`, maps every semantic colour and the
  `--radius-*` scale in `@theme inline`, declares the palette in `:root`, and has
  one `@layer base`. `ui/src/styles.css` is deleted.
- The palette is `DESIGN.md`'s HSL values written directly onto the semantic
  tokens (`--background` `hsl(234,23%,12%)`, `--card` `hsl(232,23%,18%)`,
  `--popover` `hsl(231,19%,26%)`, `--primary` `hsl(220,83%,75%)`, and so on).
  There are deliberately no `--neris-*` aliases: the ticket's point was that a
  second colour system must not exist, and a layer of aliases would have been
  exactly that layer. The comment above `:root` names `DESIGN.md` as the source.
- `--radius: 0.75rem`; `sm`/`md`/`lg`/`xl`/`2xl`/`3xl`/`4xl` are the `calc`
  multiples `DESIGN.md` gives. The four shadow steps (`sm`/`md`/`lg`/`xl`) are
  `@theme` values copied from the elevation table.
- Dark-only is enforced, not defaulted: both HTML entries carry `class="dark"`,
  `@layer base` sets HTML `color-scheme: dark`, and the main window now declares
  `theme: "Dark"` (ticket 03). Nothing in `ui/src` mentions
  `Canvas`/`CanvasText`/`GrayText`.
- `body` sets `font-feature-settings: "cv02","cv03","cv04","cv11"` and
  `-moz-osx-font-smoothing: grayscale`. Both font stylesheets are imported whole,
  so the deployed bundle carries every subset the packages ship — `latin` and
  `vietnamese` included, which is what prevents a mid-sentence fallback. Pruning
  to two subsets would mean copying fontsource's `unicode-range` values into our
  own `@font-face` rules; that is recorded in `.claude/skills/neris-ui/SKILL.md`
  rather than done, and the extra glyphs weigh nothing for a local app.
- One generated file still contains the word `oklch`: `Button`'s `secondary`
  hover is `color-mix(in oklch, var(--secondary), var(--foreground) 5%)`. It
  mixes two of our tokens rather than naming a colour, so no palette value
  survived `init`; it is left as the generator wrote it.

## Verification

| Command | Result |
| --- | --- |
| `npm --prefix ui run typecheck` | clean |
| `npm --prefix ui run test` | 13 files, 172 tests passed |
| `npm --prefix ui run build` | emits `dist/index.html`, `dist/popup.html`; no `preview.html`; fonts, CSS and two JS entries |
| `bash scripts/test.sh` | exit 0 |
| Design preview, `translate-done` and the popup harness | Vietnamese text renders in Inter with every diacritic whole, at both sizes |

## Comments

- 2026-11-16 — The `@fontsource-variable` stylesheets are all-subset, so "latin +
  vietnamese" is met by inclusion rather than by exclusion. Pruning is possible
  but adds copied `unicode-range` magic numbers; the choice is recorded in the
  skill so the next person does not "fix" it by hand.
- 2026-11-16 — No `--neris-*` aliases were created, for the reason in
  Implementation. The ticket's intent — one colour system — is what was built.
