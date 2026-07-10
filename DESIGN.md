---
name: mountOS Desktop
description: Desktop mount client for mountOS, profiles, credentials, mounts, health, diagnostics.
colors:
  background-light: "oklch(0.95 0.02 94.63)"
  background-dark: "oklch(0.07 0.005 200)"
  foreground-light: "oklch(0.12 0 0)"
  foreground-dark: "oklch(0.93 0.005 200)"
  card-light: "oklch(0.96 0.025 94.63)"
  card-dark: "oklch(0.03 0 0)"
  popover-light: "oklch(0.96 0.025 94.63)"
  popover-dark: "oklch(0.09 0.006 200)"
  primary-rust: "oklch(0.54 0.14 39)"
  primary-gold: "oklch(0.78 0.13 92)"
  secondary-light: "oklch(0.94 0 0)"
  secondary-dark: "oklch(0.15 0.005 200)"
  muted-light: "oklch(0.92 0.04 94.64)"
  muted-dark: "oklch(0.09 0.005 200)"
  muted-foreground-light: "oklch(0.42 0.05 70)"
  muted-foreground-dark: "oklch(0.55 0.01 200)"
  label-foreground-light: "oklch(0.42 0.05 70)"
  label-foreground-dark: "oklch(0.55 0.01 200)"
  accent-amber-tint: "oklch(0.92 0.06 80)"
  accent-dark: "oklch(0.16 0.008 200)"
  destructive-light: "oklch(0.54 0.24 24.42)"
  destructive-dark: "oklch(0.59 0.20 21)"
  warning-light: "oklch(0.45 0.16 55)"
  warning-dark: "oklch(0.78 0.15 75)"
  success-light: "oklch(0.49 0.17 155)"
  success-dark: "oklch(0.65 0.19 155)"
  border-light: "oklch(0.88 0 0)"
  border-dark: "oklch(0.21 0.008 200)"
  input-light: "oklch(0.96 0 0)"
  input-dark: "oklch(0.09 0.005 200)"
  ring-light: "oklch(0.61 0.14 39)"
  ring-dark: "oklch(0.78 0.13 92)"
  scrollbar-thumb-light: "oklch(0.61 0.14 39)"
  scrollbar-thumb-dark: "oklch(0.78 0.13 92)"
  pastel-mount: "oklch(0.75 0.14 90)"
  pastel-volume: "oklch(0.72 0.15 155)"
  pastel-storage: "oklch(0.72 0.15 200)"
  pastel-session: "oklch(0.72 0.15 175)"
  pastel-node: "oklch(0.72 0.15 125)"
  pastel-region: "oklch(0.72 0.15 310)"
typography:
  display:
    fontFamily: "ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif"
    fontSize: "1.875rem"
    fontWeight: 600
    lineHeight: 1.2
    letterSpacing: "-0.02em"
    fontFeature: "'cv02', 'cv03', 'cv04', 'cv11'"
  headline:
    fontFamily: "{typography.display.fontFamily}"
    fontSize: "1.5rem"
    fontWeight: 600
    lineHeight: 1.2
    letterSpacing: "-0.02em"
  title:
    fontFamily: "{typography.display.fontFamily}"
    fontSize: "1.25rem"
    fontWeight: 600
    lineHeight: 1.2
    letterSpacing: "-0.02em"
  body:
    fontFamily: "{typography.display.fontFamily}"
    fontSize: "1.125rem"
    fontWeight: 400
    lineHeight: 1.6
    letterSpacing: "-0.01em"
  body-sm:
    fontFamily: "{typography.display.fontFamily}"
    fontSize: "1.0625rem"
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: "-0.01em"
  label:
    fontFamily: "{typography.display.fontFamily}"
    fontSize: "1rem"
    fontWeight: 500
    lineHeight: 1.5
    letterSpacing: "-0.01em"
  mono:
    fontFamily: "ui-monospace, Menlo, monospace"
    fontSize: "0.7rem"
    fontWeight: 400
    lineHeight: 1.4
    letterSpacing: "0.2em"
rounded:
  sm: "0"
  md: "2px"
  lg: "4px"
  xl: "8px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "16px"
  lg: "24px"
  xl: "32px"
components:
  button-default:
    backgroundColor: "transparent"
    textColor: "{colors.foreground-light}"
    rounded: "{rounded.sm}"
    padding: "8px 16px"
    height: "36px"
  button-primary:
    backgroundColor: "{colors.primary-rust}"
    textColor: "{colors.card-light}"
    rounded: "{rounded.sm}"
    padding: "8px 16px"
    height: "36px"
  button-primary-hover:
    backgroundColor: "{colors.ring-light}"
  button-destructive:
    backgroundColor: "transparent"
    textColor: "{colors.destructive-light}"
    rounded: "{rounded.sm}"
    padding: "8px 16px"
    height: "36px"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.foreground-light}"
    rounded: "{rounded.sm}"
    padding: "8px 16px"
  button-sm:
    backgroundColor: "transparent"
    textColor: "{colors.foreground-light}"
    rounded: "{rounded.sm}"
    padding: "6px 12px"
    height: "32px"
  card:
    backgroundColor: "{colors.card-light}"
    textColor: "{colors.foreground-light}"
    rounded: "{rounded.md}"
    padding: "16px"
  input:
    backgroundColor: "{colors.input-light}"
    textColor: "{colors.foreground-light}"
    rounded: "{rounded.sm}"
    padding: "8px 12px"
    height: "36px"
---

# Design System: mountOS Desktop

**Provenance.** This system mirrors `mountos-admin-client/DESIGN.md` ("The Operator's Console"). Color, typography, spacing, and component tokens are shared across the mountOS surfaces; a token change here must be mirrored there and vice versa. Utilities in this repo live in `src/app.css` (vanilla CSS, no Tailwind).

## 1. Overview

**Creative North Star: "The Operator's Console"**

A control surface for people who mount and operate filesystems, not a marketing site. The visual language borrows from radar terminals and mission-control consoles: warm-on-cold light mode, golden-on-deep-teal dark mode, near-zero radii, corner brackets that frame data without competing with it. Density is the goal. Every pixel either carries information or stays out of the way.

The system rejects the SaaS dashboard reflex of soft cards, rounded edges, decorative gradients, and hand-holding empty states. Users are experts; the interface treats them as such. Color is rationed: rust-amber primary in light, gold in dark, red-orange destructive, and entity-coded pastels reserved for tagging objects.

**Key Characteristics:**
- Sharp geometry: `--radius` caps at 0.25rem; buttons are square. Corner-bracket frames, not soft cards.
- OKLCH-only color, perceptually uniform across light and dark. No `#fff`, `#000`, `rgb()`, or `hsl()`.
- Warm-rust primary (light) and gold primary (dark). Not blue, not teal.
- Entity-coded pastels for object tagging (one hue per noun, paired `-text` value on tints).
- The smallest UI text is 16px; body runs 17-18px.
- Flat by default; no drop shadows on containers.
- Dense data tables; no decorative whitespace.

## 2. Colors

### Primary
- **Rust Amber** (`oklch(0.54 0.14 39)`, light): interactive accent, focus ring (`--ring` `oklch(0.61 0.14 39)`), scrollbar thumb, primary buttons.
- **Console Gold** (`oklch(0.78 0.13 92)`, dark): dark-mode primary and ring. Moderate chroma, no neon glare.

### Secondary
No secondary accent by design. `--secondary` is a low-contrast neutral surface, not a brand color.

### Tertiary (entity-coded pastels)
Pastels tag domain objects only (`--pastel-mount`, `--pastel-volume`, `--pastel-storage`, `--pastel-session`, `--pastel-node`, `--pastel-region`), each with a paired `-text` value for legible labels on tints. **Identifiers, not decoration.** A mount chip is always the 90-hue gold-green; don't reassign.

### Neutral
Warm cream background (light), deep teal-black (dark), card one step from background, 1px hairline borders, muted/label text on the tinted muted-foreground tokens.

### Status
Destructive red-orange, warning amber ("needs attention", not "FYI"), success green. Hues hold steady across themes.

### Named Rules

**The Rationed Color Rule.** Primary, destructive, warning, success, and entity pastels are the only saturated families on screen. If three rust-amber elements show at once, two are wrong.

**The Entity Pastel Lock.** Pastel tokens bind to entity types one-to-one across every mountOS surface.

**The OKLCH-Only Rule.** All colors authored in OKLCH. `#fff`, `#000`, `rgb()`, `hsl()` are forbidden in product code.

## 3. Typography

System sans stack with `cv02 cv03 cv04 cv11` OpenType features; mono is `ui-monospace, Menlo, monospace`.

- **Display** 600 / 1.875rem / 1.2 / -0.02em: one per view (the topbar title).
- **Headline** 600 / 1.5rem / 1.2 / -0.02em: card and section heads.
- **Title** 600 / 1.25rem / 1.2 / -0.02em: subsection labels inside a panel.
- **Body** 400 / 1.125rem / 1.6 / -0.01em: default prose.
- **Body Small** 400 / 1.0625rem / 1.5: table cells, dense lists.
- **Label** 500 / 1rem / 1.5: button text, form labels. 16px is the floor.
- **Mono Microlabel** 400 / 0.7rem / 0.2em tracking, uppercase: table column headers and technical metadata labels only.

### Named Rules

**The 16px Floor Rule.** No UI text below 16px. The only exception is the 0.7rem mono microlabel on table headers and technical metadata, which is a label, not prose.

**The Mono-For-Microlabels-Only Rule.** Mono is reserved for table column headers, technical IDs, paths, and command previews. Forbidden in body copy, button labels, headings, and as decorative section eyebrows. One mono microlabel above a code/command block is technical metadata; a mono eyebrow above every section is banned scaffolding.

## 4. Elevation

Flat by default. Depth is a one-step background lift (background → card) plus 1px hairline borders.

**The No-Drop-Shadow Rule.** Containers do not float. If a surface needs lift, change its tone, not its shadow.

**The Glow-Is-State Rule.** Glow signals interaction (focus ring, LED status dots), never decoration. The `.led` pulse respects `prefers-reduced-motion`.

## 5. Components

All utilities live in `src/app.css`.

### Buttons (`.btn`)
- Square (`border-radius: 0`), 1px border, transparent fill; hover swaps to `--accent` background; active scales 0.98 with a 10% primary tint.
- `.primary`: solid `--primary` with `--primary-foreground` text. `.destructive`: outline-only destructive border/text, 10% tint on hover; no filled destructive variant. `.ghost`: transparent border. `.icon-btn`: 36px square; every icon-only button carries an `aria-label`.
- Heights: 36px default; interactive elements in dense tables keep at least 36px with honest hit areas.

### Cards / Panels (`.surface`, `.panel`)
- 1px hairline `--border`, `--card` background, 2px radius, no shadow. Cards do not nest; sub-regions use a divider or `--accent` tint.
- `.corner-brackets` (two-corner variant, `--ring` tinted) frames THE primary content panel of a view. One bracketed surface per view maximum.

### Inputs (`.input`, `.select`, `.textarea`)
- `--input` background, 1px border, square, 36px height matching buttons. Focus is the global 2px `--ring` outline at 2px offset. Secrets are paste-first password fields and are never echoed.

### Tables (`.table`)
- Dense rows, 17px cells, 1px row separators. Column headers are the mono microlabel (0.7rem, 0.2em tracking, uppercase) on `--label-foreground`.

### Status
- `.badge` outline chips; `.badge.success|warning|destructive` tint border and text with the status color; `.badge.mount` uses the mount entity pastel.
- `.led` 8px status dot with a 2.5s opacity pulse, disabled under reduced motion.

### Dialogs
- Native `<dialog>` only (focus trap, Escape, `aria-modal` for free). One purpose per dialog; reserved for flows that must interrupt (secret entry). Everything else is inline disclosure.

### Sanctioned patterns (NOT "decorative gradients")
- Corner-bracket marks, the LED pulse, and the brand mark's clip-path chamfer are structural signature, keep them.
- `.tech-grid`, the dual `linear-gradient` 20px grid backdrop on empty states. Brand backdrop, keep.
- `.skeleton`, the opacity-pulse loading placeholder (no shimmer gradient); collapses under reduced motion.
- Any NEW gradient, glow, or clip-path must earn an explicit entry here. Default-deny.

## 6. Do's and Don'ts

### Do:
- **Do** author every color in OKLCH via the tokens; keep light and dark parity for every new token.
- **Do** keep the page flat; separation via tone or hairline border.
- **Do** use entity pastels consistently for the same noun across views.
- **Do** keep body type at 17-18px, headings tightened to -0.02em.
- **Do** show the exact CLI command in mono where the app runs one (command preview, fix commands).
- **Do** respect `prefers-reduced-motion` globally.
- **Do** give every icon-only control an `aria-label`, every view a keyboard path, and dialogs native focus semantics.

### Don't:
- **Don't** use rounded, bubbly, or playful aesthetics; radius caps at 0.25rem.
- **Don't** add gradients, glassmorphism, decorative illustrations, or marketing hero sections.
- **Don't** use `border-left`/`border-right` stripes thicker than 1px as accents.
- **Don't** build hero-metric tiles (big number + small label grids). Put counts inline in panel heads; users want tables.
- **Don't** put mono in headings, buttons, or as section eyebrows; microlabels belong to tables and technical metadata only.
- **Don't** nest cards, and don't reach for a modal when inline disclosure works.
- **Don't** drop UI text below 16px.
- **Don't** use em dashes anywhere in product copy. Use commas, colons, semicolons, or periods.
- **Don't** introduce a secondary brand color; rust → gold is the entire palette story.
- **Don't** reference internal binary names in user-facing copy; the public name is `mountos` and the product is "mountOS Desktop".
