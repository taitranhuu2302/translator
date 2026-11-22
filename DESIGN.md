# Neris — Design System

> Visual design reference for the Neris desktop app. Covers themes, colors, typography, spacing, components, and overlay styles. Architecture and code structure are documented separately.

---

## Brand Identity

| Attribute | Value |
| --- | --- |
| **Brand** | Neris — the app, the mark and this design system share one name |
| **Tagline** | Vietnamese ↔ English translation, always at your fingertips |
| **Philosophy** | Imagination meets precision engineering — ocean depth, fluid movement, crafted warmth |
| **Motto** | Dream like an artist. Build like an engineer. Finish like a craftsman. |
| **Creative Source** | The ocean — depth, mystery, endless discovery |
| **Theme** | **Dark-only** — no light mode. Designed for meetings, media consumption, and low-light environments |

---

## Brand Assets

| Asset | File | Use |
| --- | --- | --- |
| Lockup — mark and wordmark | `docs/assets/neris-lockup.png` | README and external material |
| App icon source | `crates/app/icons/icon.svg`, rendered to a 1024 px `icon.png` (8-bit RGBA — Tauri embeds the rows raw) | Bundle icon, packaged as `Neris.icns` |
| Menu bar mark | `crates/app/icons/tray-template.svg`, rendered to `tray-template.png` | Status item, set `icon_as_template(true)` |
| In-app mark | `ui/src/components/NerisMark.tsx` | Onboarding and the Settings header |

The lockup keeps the brand blue `#0042CC` of the source artwork. That blue is an asset colour, not a UI token: nothing in `ui/src/index.css` uses it, and the interface itself stays on the Neris Ocean palette below.

The app icon is the white mark on the ocean gradient, inside macOS's rounded-square tile with the transparent margin the Dock expects. The menu bar takes the black template PNG instead, so the system tints it to match the menu bar rather than showing a white square.

---

## Color System — Neris Ocean Palette

### Conceptual Palette

| Token | Hex | RGB | UI Role |
| --- | --- | --- | --- |
| `--neris-deep-ocean` | `#0d1129` | `(13,17,41)` | Primary backgrounds, deepest surface layer |
| `--neris-ocean-navy` | `#141c36` | `(20,28,54)` | Logo, headers, navigation bars, elevated surfaces |
| `--neris-marine-blue` | `#2c334a` | `(44,51,74)` | Secondary UI elements, card surfaces, subtle borders |
| `--neris-horizon-blue` | `#3d5993` | `(61,89,147)` | Interactive elements, hover states, links |
| `--neris-aurora-blue` | `#a0c4f7` | `(160,196,247)` | CTAs, active/live indicators, recording state |

### Actual Implementation (Map to shadcn/ui tokens)

The app maps these conceptual tokens to shadcn/ui's semantic design token system:

| shadcn Token | Value (HSL) | Conceptual Token | Purpose |
| --- | --- | --- | --- |
| `--background` | `hsl(234, 23%, 12%)` | Deep Ocean | Main window body |
| `--foreground` | `hsl(228, 70%, 88%)` | — | Primary text |
| `--card` | `hsl(232, 23%, 18%)` | Ocean Navy | Card surfaces, elevated panels |
| `--card-foreground` | `hsl(228, 70%, 88%)` | — | Card text |
| `--popover` | `hsl(231, 19%, 26%)` | Marine Blue | Dropdowns, popovers |
| `--popover-foreground` | `hsl(228, 70%, 88%)` | — | Popover text |
| `--primary` | `hsl(220, 83%, 75%)` | Aurora Blue | Primary buttons, active states, recording indicator |
| `--primary-foreground` | `hsl(234, 23%, 12%)` | Deep Ocean | Text on primary |
| `--secondary` | `hsl(231, 19%, 26%)` | Marine Blue | Secondary buttons, subtle surfaces |
| `--secondary-foreground` | `hsl(228, 28%, 73%)` | — | Secondary text |
| `--muted` | `hsl(236, 25%, 13%)` | Deep Ocean (darkened) | Muted backgrounds |
| `--muted-foreground` | `hsl(228, 28%, 73%)` | — | Muted/deemphasized text |
| `--accent` | `hsl(220, 83%, 75%)` | Aurora Blue | Accent elements |
| `--accent-foreground` | `hsl(0, 0%, 100%)` | — | Text on accent |
| `--destructive` | `hsl(350, 74%, 73%)` | Warm Red | Delete, danger actions |
| `--destructive-foreground` | `hsl(0, 0%, 100%)` | — | Text on destructive |
| `--border` | `hsl(234, 19%, 26%)` | Marine Blue (subtle) | Borders, dividers |
| `--input` | `hsl(231, 16%, 34%)` | — | Input fields border |
| `--ring` | `hsl(220, 83%, 75%)` | Aurora Blue | Focus rings |

### Sidebar Palette

| Token | Value (HSL) | Purpose |
| --- | --- | --- |
| `--sidebar` | `hsl(236, 25%, 13%)` | Sidebar background (deeper than main) |
| `--sidebar-foreground` | `hsl(228, 40%, 80%)` | Sidebar text/icons |
| `--sidebar-primary` | `hsl(220, 83%, 75%)` | Active nav item indicator |
| `--sidebar-primary-foreground` | `hsl(0, 0%, 100%)` | Text on active nav |
| `--sidebar-accent` | `hsl(234, 19%, 24%)` | Hover state in sidebar |
| `--sidebar-accent-foreground` | `hsl(228, 70%, 88%)` | Hover text in sidebar |
| `--sidebar-border` | `hsl(232, 23%, 18%)` | Sidebar edges |
| `--sidebar-ring` | `hsl(220, 83%, 75%)` | Focus ring in sidebar |

### Chart Colors (for data viz in sessions, statistics)

| Token | HSL | Swatch |
| --- | --- | --- |
| `--chart-1` | `hsl(220, 83%, 75%)` | Aurora Blue |
| `--chart-2` | `hsl(170, 47%, 69%)` | Teal |
| `--chart-3` | `hsl(40, 69%, 78%)` | Warm Gold |
| `--chart-4` | `hsl(355, 71%, 77%)` | Soft Red |
| `--chart-5` | `hsl(270, 81%, 80%)` | Purple |

---

## Typography

### Font Stack

| Role | Font | Weights |
| --- | --- | --- |
| **Primary UI** | Inter (variable) | 400, 500, 600, 700 |
| **Monospace** | JetBrains Mono / SF Mono | 400, 500 |
| **Fallback** | `-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif` | — |

### Font Features

```
font-feature-settings: "cv02", "cv03", "cv04", "cv11";
```

Inter's stylistic alternates for a more refined, modern look.

### Scale

| Class | Size | Weight | Usage |
| --- | --- | --- | --- |
| `text-xs` | 12px | 400 | Footer text, legal, meta labels |
| `text-sm` | 14px | 400 | Descriptive text, secondary info |
| `text-base` | 16px | 400–500 | Body text, labels, inputs |
| `text-lg` | 18px | 500–600 | Card titles, section headers |
| `text-xl` | 20px | 600 | Page titles |
| `text-2xl` | 24px | 600–700 | Major section headings |
| `text-3xl` | 32px | 700 | Overlay transcript (large) |
| `text-4xl` | 36px+ | 700 | Hero text, brand |

### Line Height

| Context | Value | Rationale |
| --- | --- | --- |
| Body text | `1.5` | Comfortable reading |
| Headings | `1.3` | Tighter for visual grouping |
| Bilingual transcript lines | `1.6` | Extra room for Vietnamese diacritics (ắ, ệ, ỏ, ư, ơ) |

### Font Smoothing

```css
body {
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}
```

Applied globally via Tailwind's `antialiased` utility.

---

## Border Radius

| Token | Value | Usage |
| --- | --- | --- |
| `--radius` (base) | `0.75rem` (12px) | macOS-native feel, consistent with window corner radius |
| `--radius-sm` | `calc(var(--radius) * 0.6)` ≈ 7px | Buttons, small cards, dropdown items |
| `--radius-md` | `calc(var(--radius) * 0.8)` ≈ 10px | Cards, panels, dialogs, select fields |
| `--radius-lg` | `var(--radius)` = 12px | Modals, sidebar panels, larger surfaces |
| `--radius-xl` | `calc(var(--radius) * 1.4)` ≈ 17px | Main window, overlay background |
| `--radius-2xl` | `calc(var(--radius) * 1.8)` ≈ 22px | Extra large surfaces |
| `--radius-3xl` | `calc(var(--radius) * 2.2)` ≈ 26px | Hero cards, feature panels |
| `--radius-4xl` | `calc(var(--radius) * 2.6)` ≈ 31px | Maximum radius |

Full circles/pills use Tailwind's `rounded-full` (9999px).

---

## Shadows & Depth

### Elevation Scale

| Level | Shadow | Tailwind | Usage |
| --- | --- | --- | --- |
| `sm` | `0 1px 4px hsla(0,0%,0%,0.08)` | `shadow-sm` | Cards, dropdowns |
| `md` | `0 2px 8px hsla(0,0%,0%,0.12)` | `shadow-md` | Dialogs, modals |
| `lg` | `0 4px 16px hsla(0,0%,0%,0.16)` | `shadow-lg` | Overlay window, floating panels |
| `xl` | `0 8px 24px hsla(0,0%,0%,0.20)` | `shadow-xl` | Top-level popovers, tooltips |

### Design Intent

- Subtle multi-layer shadows — not flat design
- Surface elevation communicated through shadow intensity, not heavy borders
- Depth comes from layered dark blues + soft shadows
- Minimal chrome — no bevels, no heavy strokes

---

## Glassmorphism (Frosted Glass)

> **Default: clean solid surfaces.** Glassmorphism is optional, used sparingly for special surfaces where depth adds value.

### Glass Surface Recipe

| Property | Value |
| --- | --- |
| Background opacity | 60% of Ocean Navy (`hsla(var(--neris-ocean-navy-hsl), 0.6)`) |
| Blur | `backdrop-blur-xl` (24px) |
| Border (top) | `1px hsla(255,255%,255%,0.08)` — subtle frosted edge highlight |
| Shadow | Multi-layer (see Shadows & Depth) |

### Where to Use

- Overlay window
- Active state panels
- Subtle surface differentiation

### Where NOT to Use

- Standard cards and dialogs
- Input fields
- Navigation

---

## macOS Design Language

The app takes strong visual cues from macOS as the primary design target:

| Aspect | Implementation |
| --- | --- |
| **Title bar** | Unified toolbar (no separate title bar). Traffic light controls respected on macOS. |
| **Corner radius** | 12px base (`--radius`), matching native macOS window radius |
| **Chrome** | Minimal — depth from shadows + color layering, not borders/bevels |
| **Spacing gutters** | 16–24px (generous, macOS-native feel) |
| **Scrollbars** | Overlay scrollbars (thin, auto-hide) via `ScrollArea` component |
| **Animations** | Spring-style easing (0.4–0.6s `ease-out`) for reveals; 0.2s `ease` for micro-interactions |
| **Icons** | Lucide outlined — thin, consistent stroke width, matching SF Symbols weight |
| **Dark-only** | All surfaces are dark; no light mode toggle exists |

---

## Overlay Window Styles

The overlay is a separate Tauri window with its own stylesheet.

### Unique Overlay Aspects

| Property | Value |
| --- | --- |
| Background | Transparent (`background: transparent`) |
| Body overflow | Hidden (`overflow: hidden`) |
| Window frame | Borderless, always-on-top |
| Draggable | User can reposition anywhere on screen |
| Scroll | Auto-scrolls to latest content |

### Overlay Color Tokens (subset of main palette)

| Token | Value | Usage |
| --- | --- | --- |
| `--background` | `hsl(234, 23%, 12%)` | Text background container |
| `--foreground` | `hsl(228, 70%, 88%)` | Source transcript text |
| `--card` | `hsl(234, 21%, 18%)` | Text container background |
| `--card-foreground` | `hsl(228, 70%, 88%)` | Translated text |
| `--primary` | `hsl(220, 83%, 75%)` | Accent line, speaker labels |
| `--muted-foreground` | `hsl(228, 28%, 73%)` | Timestamps, meta text |
| `--border` | `hsl(234, 19%, 26%)` | Subtle separators |

### Overlay Configuration (user-adjustable via Settings)

| Setting | Default | Range |
| --- | --- | --- |
| Font size | 26px | 16–48px |
| Line height | 1.35 | 1.0–2.0 |
| Width | 720px | 400–1200px |
| Height | 200px | 100–600px |
| Theme | Dark | Dark / Light |
| Opacity | 88% | 30–100% |
| Show translation | On | On / Off |

---

## Spacing & Layout

### Gutters & Padding

| Scale | Value | Usage |
| --- | --- | --- |
| `p-2` / `gap-2` | 8px | Compact icons, inline groups |
| `p-3` / `gap-3` | 12px | Card inner padding (compact) |
| `p-4` / `gap-4` | 16px | Standard panel padding, card content |
| `p-6` / `gap-6` | 24px | Section gutters, page margins |
| `p-8` | 32px | Generous whitespace, hero sections |

---

## Component Styling Patterns

### Buttons

| Variant | Purpose | Style |
| --- | --- | --- |
| `default` | Primary CTAs | `bg-primary text-primary-foreground hover:bg-primary/90` |
| `secondary` | Alternative actions | `bg-secondary text-secondary-foreground hover:bg-secondary/80` |
| `outline` | Cancel, subtle | `border border-input bg-background hover:bg-accent hover:text-accent-foreground` |
| `ghost` | Icon-only, toolbar | `hover:bg-accent hover:text-accent-foreground` |
| `destructive` | Delete, danger | `bg-destructive text-destructive-foreground hover:bg-destructive/90` |

### Inputs

```
border border-input
bg-transparent
rounded-md
focus:ring-2 focus:ring-ring focus:ring-offset-2
```

### Cards

```
bg-card text-card-foreground
rounded-lg
shadow-sm
border border-border/40
```

### Dialogs / Modals

```
bg-background
rounded-lg
shadow-lg
border border-border/40
```

### Status States (all components)

| State | Treatment |
| --- | --- |
| **Idle** | Default styles |
| **Hover** | Slightly lighter background (`hover:bg-accent/50`) or subtle border highlight |
| **Active** | Primary color accent (Aurora Blue `--primary`) |
| **Disabled** | `opacity-50 cursor-not-allowed` |
| **Loading** | Skeleton components (pulse animation) or spinner |
| **Error** | Destructive color ring/border + error message text |
| **Empty** | Centered muted text with optional illustration/icon |

---

## Animation & Motion

### Timing

| Type | Duration | Easing | Usage |
| --- | --- | --- | --- |
| Micro-interactions | 150–200ms | `ease` | Hover states, focus rings, toggle switches |
| Panel reveals | 400–600ms | `ease-out` | Sidebar expand/collapse, modal open/close |
| Content appearance | 300ms | `ease-out` | New transcript lines, notification toasts |
| Loading indicators | Continuous | `linear` (infinite) | Progress bars, skeleton pulses, spinners |

### Prefers Reduced Motion

Respect `prefers-reduced-motion: reduce` media query — disable all non-essential animations.

### Tailwind Animation Classes

Uses `tw-animate-css` plugin for shadcn/ui animations (dropdown enter/exit, dialog open/close, accordion expand/collapse, etc.).

---

## Icons — Lucide

| Usage | Examples |
| --- | --- |
| **Brand/Logo** | `NerisMark` — the app's own N mark (`ui/src/components/NerisMark.tsx`), not a Lucide icon |
| **Navigation** | `Home`, `Archive`, `Settings` |
| **Actions** | `Play`, `Square` (stop), `Eye` (show overlay), `Trash2` (delete), `Download` (model), `FolderPlus` |
| **Status** | `Check`, `AlertTriangle`, `X`, `Loader` |
| **UI** | `ChevronDown`, `ChevronRight`, `Search`, `X`, `Menu`, `Ellipsis` |

All icons: outlined style, consistent 2px stroke width, 16–24px sizes (`h-4 w-4` to `h-6 w-6`).

---

## Platform Adaptations

| Aspect | macOS | Windows |
| --- | --- | --- |
| Title bar | Native traffic light controls + unified toolbar | Custom title bar buttons |
| Font rendering | `-webkit-font-smoothing: antialiased` | `-moz-osx-font-smoothing: grayscale` |
| Scrollbars | Overlay (thin, auto-hide) | Platform native |
| Corner radius | 12px (matches native) | 12px (consistent) |
| Window frame | Native shadow + rounded corners | App-managed frame |
| Icons | Lucide (cross-platform consistent) | Lucide (cross-platform consistent) |

---

## Visual Hierarchy — Color Usage Guide

```
Deep Ocean (#001A33 / hsl(234,23%,12%))   → Background
Ocean Navy (#003366 / hsl(232,23%,18%))    → Cards, surface elevation
Marine Blue (#004080 / hsl(231,19%,26%))   → Borders, secondary buttons, dividers
Horizon Blue (#0059B3)                     → Interactive hover, focus rings
Aurora Blue (#0066CC / hsl(220,83%,75%))   → Primary CTAs, active states, "live" indicators
```

Each deeper blue creates a layer of elevation, building depth without heavy strokes or harsh borders.