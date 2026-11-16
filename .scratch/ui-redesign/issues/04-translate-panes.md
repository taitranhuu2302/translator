# 04: The two panes

**What to build:** The Translate and Improve panes become one frame in two roles: two bordered cards, 16–24px gutters, a small uppercase label each, and the result pane's actions in its own header. Loading, empty and error states use the new vocabulary, and the invisible primary-button hover is fixed.

**Blocked by:** 02, 03

**Status:** resolved

- [x] `1fr 1fr` grid with design-system gutters; each pane a card surface with a label.
- [x] The source pane is a `Textarea` with a visible focus ring; it is no longer borderless with `outline: none`.
- [x] The result pane's actions (Copy, Replace, Read Aloud — whatever exists today) sit in its header, not at the bottom.
- [x] Busy state: a spinner inside the action button, and the pane content stays in place.
- [x] Loading state: a skeleton in the shape of the coming answer, so the layout does not jump.
- [x] Empty and error states render through `EmptyState` and `Alert`; wording still comes only from `notice.ts`.
- [x] Primary button hover is visible; no rule sets a transparent border that hover cannot change.
- [x] Every control has a `:focus-visible` ring in `--ring`.
- [x] The harness shows both modes with short, long, loading, empty and failed content.
- [x] Typecheck, tests and build pass.

## Implementation

- `ui/src/components/TranslatePanes.tsx` is the one frame in two roles. The grid
  is `grid-cols-2` (CSS's `repeat(2, minmax(0,1fr))`) over `gap-4 p-4`: the
  `minmax(0, …)` is what `1fr 1fr` needs to stop a long word widening its column,
  and 16px is the gutter `DESIGN.md` gives a dense surface. Each pane is
  `rounded-xl border border-border/70 bg-card shadow-sm`.
- `PaneHeader` draws the small uppercase label (`text-[0.6875rem]`, tracking
  `0.08em`) and takes the header's children, so the result pane can put its
  actions there. The actions are Copy (`Copy`/`Check`) and Swap
  (`ArrowLeftRight`), both `size="xs"` ghost buttons; disabled by the existing
  `canSwap` rule rather than by a new one.
- The source pane is the installed `Textarea`, `flex-1 resize-none border-0
  bg-transparent`, with the focus ring moved to the card:
  `focus-within:border-ring/50 focus-within:ring-3 focus-within:ring-ring/15`.
  The field itself has `focus-visible:ring-0` so there are not two rings.
- Busy: `LoaderIcon` inside the action button (`animate-spin`), and the panes are
  not unmounted or re-keyed, so nothing moves when the answer arrives.
- Loading: `AnswerSkeleton` — four `Skeleton` lines at 94/88/91/52% width, `bg-foreground/10` — drawn in the result pane's answer slot. The
  empty and failed states share the same slot, so the pane's height is the same
  in every phase.
- Empty renders `EmptyState` (`ui/src/components/EmptyState.tsx`, written here:
  muted icon, sentence, optional detail); failure renders `NoticeView`, which is
  `Alert` + `AlertTitle` + `AlertDescription` with the icon and tint per kind. No
  sentence in either file was written by the component — the titles and details
  come from `ui/src/notice.ts` through `noticeFor`, and the popup renders the
  same `NoticeView`.
- Primary hover is the primitive's `hover:bg-primary/80` on an opaque colour, and
  the button's base is `border border-transparent` — a *colour*-less border, so
  no rule can hold a transparent border that hover cannot repaint.
- Copy in Improve mode exists twice (corrected, better) with the same focus-visible
  ring; `formatShortcut` output in the pane placeholder reuses `Kbd` elsewhere,
  not a styled span.

## Verification

| Command | Result |
| --- | --- |
| `npm --prefix ui run typecheck` | clean |
| `npm --prefix ui run test` | 13 files, 172 tests passed (10 of them in `translateWindow.test.ts`) |
| `npm --prefix ui run build` | clean |
| `bash scripts/test.sh` | exit 0 |
| Design preview: `translate-empty`, `-busy`, `-done`, `-long`, `-failure`, `-offline`; `improve-empty`, `-busy`, `-done`, `-failure` | each state draws in place; the long answers scroll in the result pane only |
| Design preview at 680×460 | both panes stay side by side; the long Improve answer scrolls (262px visible, 344px content) and no control clips |

## Comments

- 2026-11-16 — The provider badge (`via Azure`) lives in the result header. It
  was not invented here: ticket 03 wanted a provider name somewhere, and the
  answer is the only place the name is a fact.
- 2026-11-16 — At the minimum window the "Enter translates · Shift+Enter adds a
  line" hint is hidden by a container query instead of truncating. The keys are
  in the footer regardless, and a half-sentence read worse than a gap.
