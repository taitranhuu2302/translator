---
name: neris-ui
description: Conventions for building UI in this repo — Tailwind v4 + shadcn tokens, dark-only rules, bundled fonts, Quick Translate popup limits, and how to add a component. Use whenever editing anything under ui/ or adding a control.
---

# Neris UI

The Translate Window and the Quick Translate popup share one design system built
on Tailwind CSS v4 and shadcn/ui. `DESIGN.md` is the visual source of truth:
tokens are its values, not approximations.

## Where things live

- `ui/src/index.css` — the only stylesheet. `@import "tailwindcss"`, the `@theme`
  token block, and one `@layer base`.
- `ui/src/components/ui/` — shadcn components, generated then edited in place.
- `ui/src/components/` — the app's own components.
- `ui/src/preview/` + `ui/preview.html` — the dev-only harness.
- `ui/components.json` — shadcn's project config.

## Adding a component

```bash
npx shadcn@latest add <name> -c ui
```

`-c ui` is required: the repository root is an npm workspace and the CLI refuses
to run there. The same applies to `info` and `view`.

The shadcn MCP server is configured in `.mcp.json`. With `components.json`
present, its registry tools work directly.

## Rules

1. **Dark-only.** `<html class="dark">` is permanent. Never add a light branch, a
   `dark:` variant, or a theme toggle. Delete `Canvas`/`CanvasText`/`GrayText`
   rather than porting it.
2. **Tokens only.** No hex, no `rgba()`, no arbitrary colour value. If a colour
   is missing, add it to `@theme` — never inline it in a component.
3. **Radius from `--radius`.** Use `rounded-sm|md|lg|xl`; the scale already
   matches `DESIGN.md`.
4. **Fonts carry the Vietnamese subset.** Inter and JetBrains Mono come from
   `@fontsource-variable/*`. Their stylesheets ship every subset the package
   has — `latin`, `vietnamese` and the rest — so diacritics never fall back
   mid-sentence; the subsets are not pruned, because doing that means copying
   fontsource's `unicode-range` values into our own `@font-face` rules. Any face
   added later must include `vietnamese` too.
5. **The popup takes no Radix component.** It is 420px wide, so a portal
   dropdown clips; compose it from native elements styled with Tailwind. It *does*
   receive keys — ADR-0005 has it become key and hand first responder to the web
   view — so this is a layout rule, not a focus rule.
6. **One stylesheet.** `ui/src/styles.css` is gone. Do not reintroduce a second
   system, a CSS module, or inline styles for anything the tokens cover.
7. **Failure copy comes from `notice.ts`.** A component renders the notice model;
   it never writes a sentence about what went wrong.
8. **Motion follows the scale.** 150–200ms `ease` for micro-interactions, 300ms
   `ease-out` for content appearing, 400–600ms `ease-out` for panels. Honour
   `prefers-reduced-motion`.
9. **Icons are Lucide**, outlined, 2px stroke, 16–24px. No mixed icon sets.
10. **Every state has a rendering.** Loading is a skeleton in the shape of the
    coming content; empty is `EmptyState`; failure is `Alert` with an icon and a
    colour per cause. An empty box is a bug.

## Checking your work

```bash
npm run typecheck && npm run test && npm run build
```

Visual change: update `preview.html` first, then the app. Anything that needs a
human eye gets a table under `.scratch/<feature>/checks/`; the agent builds and
signs the bundle but **never launches it** — the user runs it.
