# Design

<!-- impeccable:design-schema 1 -->

Design system for CamDrop (see [PRODUCT.md](PRODUCT.md)). Native desktop, Operate surface: restrained, task-first, high legibility. Light background, lake-blue accent.

## Non-negotiable rules

1. **Legibility first.** Every piece of text must be clearly readable against its background. Never ship low-contrast text.
2. **No light text on light backgrounds.** Never let text render white (or near-white) on `SURFACE`, `SURFACE_ALT`, `SURFACE_HI` or `BG`.
3. **`.strong()` text must be explicitly colored.** In egui, `strong_text_color()` is derived from `Visuals::widgets.active.fg_stroke`. Because selected widgets need white text, that field is white — so any bare `.strong()` label turns white and disappears on light surfaces. Use the `heading()` helper (`RichText::new(text).strong().color(TEXT)`) for every dark heading, or add an explicit `.color(...)`.
4. **Accent is for state and action only.** `ACCENT` marks the primary action, the current step, the current selection and progress. It is not used for decoration or for ordinary headings/counts.
5. **Contrast targets.** Body/labels ≥ 4.5:1 against their background; large or bold text ≥ 3:1. When unsure, darken the text.

## Palette

| Token | Value | Use |
|---|---|---|
| `BG` | `#F4F6F9` | window backdrop |
| `SURFACE` | `#FFFFFF` | content surface (preview, cards, log) |
| `SURFACE_ALT` | `#EEF2F6` | side panel / bars (second neutral layer) |
| `SURFACE_HI` | `#E2E8F0` | idle control fill, day cells |
| `BORDER` | `#E2E8F0` | 1px separators and outlines |
| `TEXT` | `#1F2933` | primary text (high contrast on all surfaces) |
| `TEXT_WEAK` | `#64748B` | secondary text (still ≥ 4.5:1 on white) |
| `ACCENT` | `#2AA6D0` | primary action, selection, current step, progress |
| `ACCENT_DARK` | `#1C80A8` | accent text on light surfaces (thresholds, month labels) |
| `ON_ACCENT` | `#FFFFFF` | text on an `ACCENT` fill only |

## Typography

One family (system CJK sans + default Latin). Fixed scale, tight ratio (~1.15):

| Role | Size |
|---|---|
| Screen/panel title | 20 |
| Section title | 14–16 |
| Body / labels / buttons | 14 |
| Small / hints / chips | 11.5–12.5 |
| Monospace (log, paths) | 13 |

## Layout tokens

- Spacing scale: `4 / 8 / 12 / 16 / 24`. Panel inner margin `12`; region gap `16`.
- Radii: controls `8`, cards/bars `6–8`, window `12`.
- Control heights: primary `46–48`, secondary/继续 `42`, step header `38`, chips `22–34`.
- Panels on `SURFACE_ALT`; content on `SURFACE`; separate regions by the neutral layer, not by borders alone.

## Components

- **Step accordion**: all four headers visible; the current step is `ACCENT`-filled (white text), reachable steps `SURFACE_HI`, future steps dimmed and non-interactive. Instant switching, no fade.
- **Buttons**: hand-drawn full-width buttons with hover (darken) and press (darken more) feedback plus a pointer cursor. Disabled = `SURFACE_HI` fill, muted text.
- **Date filter**: each **year is its own collapsible box** (bordered `SURFACE` card, roomy inner margin, `CollapsingHeader`), with a "select all" text control inside; month = a dot + `ACCENT_DARK` label header on its own line; day = small outlined white cells, `ACCENT`-filled when selected. Year sits a level below the section headings and is not accent-heavy.
- **Preview**: `egui_extras` table, striped, `TEXT` headers, truncated cells with tooltips.
- **Status bar**: pinned bottom; result/phase left, progress right, log toggle far right.

## Known egui pitfalls encoded here

- `strong_text_color()` follows `widgets.active.fg_stroke`. Keep `widgets.active.fg_stroke = TEXT` dark, and use `heading()` for headings. Do not rely on bare `.strong()`.
- Do **not** nest a `Panel` inside a resizable side panel: egui stores the panel's content rect as its size, so the nested panel's full width plus the outer frame margin grows the panel every frame (a runaway feedback that pushes the content sideways). The primary action is instead placed at the top of the side panel and the workflow scrolls below it; the side panel is also capped with `max_size`.
