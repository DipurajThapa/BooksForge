# Design System — BooksForge

**Version:** 1.0.0  •  **Date:** 2026-05-06  •  **Authoritative for visual tokens used in `packages/ui/`.**

This document specifies the design tokens and component conventions Claude Code must implement in `packages/ui/`. It is the visual contract for every screen in `UI_UX_SPEC.md`. Typography, colour, spacing, and motion are defined here; component behaviour is defined in `UI_UX_SPEC.md`.

**Scope.** MVP screens only. Collaboration, marketplace, and mobile surfaces are post-MVP.

---

## 1. Design principles

BooksForge is a tool for sustained deep work. The interface should:

- **Disappear when writing.** The editor surface must have no competing visual noise.
- **Signal clearly when the machine is doing something.** Agent runs, saves, exports are never silent.
- **Feel warm, not clinical.** Warm neutral tones, a humanist typeface. This is not a code editor.
- **Respect the writer's OS preference.** Full dark and light mode; system default on first launch.

---

## 2. Colour system

Tokens are CSS custom properties. They are set on `:root` (light) and `[data-theme="dark"]` (dark).

### 2.1 Neutral scale (warm grey — base for all surfaces)

| Token | Light | Dark | Usage |
|-------|-------|------|-------|
| `--color-surface-0` | `#FAFAF8` | `#171714` | App background |
| `--color-surface-1` | `#F3F3EF` | `#1F1F1C` | Sidebar, panels |
| `--color-surface-2` | `#EAEAE5` | `#27271F` | Card, popover backgrounds |
| `--color-surface-3` | `#DEDED8` | `#313128` | Hover states, dividers |
| `--color-surface-4` | `#C8C8C0` | `#3E3E34` | Borders, input outlines |

### 2.2 Text

| Token | Light | Dark | Usage |
|-------|-------|------|-------|
| `--color-text-primary` | `#1A1A17` | `#F0F0EC` | Body copy, headings |
| `--color-text-secondary` | `#5A5A52` | `#A0A097` | Labels, captions, metadata |
| `--color-text-tertiary` | `#8A8A80` | `#68685E` | Placeholders, disabled |
| `--color-text-inverse` | `#FAFAF8` | `#1A1A17` | Text on accent backgrounds |

### 2.3 Accent (amber — used sparingly for actions and focus rings)

| Token | Value | Usage |
|-------|-------|-------|
| `--color-accent-500` | `#D97706` | Primary buttons, active states |
| `--color-accent-400` | `#F59E0B` | Focus rings, highlights |
| `--color-accent-300` | `#FCD34D` | Hover on accent surfaces |
| `--color-accent-600` | `#B45309` | Pressed state on primary buttons |
| `--color-accent-100` | `#FEF3C7` | Accent-tinted backgrounds (light mode) |
| `--color-accent-900` | `#451A03` | Accent-tinted backgrounds (dark mode) |

### 2.4 Semantic colours

| Token | Light | Dark | Usage |
|-------|-------|------|-------|
| `--color-success` | `#16A34A` | `#4ADE80` | Validation passed, saved |
| `--color-warning` | `#D97706` | `#FCD34D` | Validator warnings |
| `--color-error` | `#DC2626` | `#F87171` | Validation errors, destructive actions |
| `--color-info` | `#2563EB` | `#60A5FA` | Status messages, agent progress |

### 2.5 Editor surface (special — overrides surface-0 in focus mode)

| Token | Light | Dark |
|-------|-------|------|
| `--color-editor-bg` | `#FFFEF9` | `#141411` |
| `--color-editor-text` | `#1A1A17` | `#EEEEEa` |
| `--color-editor-selection` | `rgba(217,119,6,0.18)` | `rgba(217,119,6,0.28)` |

---

## 3. Typography

### 3.1 Font families

| Token | Value | Usage |
|-------|-------|-------|
| `--font-prose` | `"Lora", "Georgia", serif` | Editor body text, chapter titles |
| `--font-ui` | `"Inter", system-ui, sans-serif` | All UI chrome (sidebar, toolbar, panels) |
| `--font-mono` | `"JetBrains Mono", "Fira Code", monospace` | Code blocks in editor, IDs, paths |

Fonts are self-hosted under `apps/desktop/src-ui/src/assets/fonts/`. Do not load from a CDN — the app is offline-first.

### 3.2 Type scale

| Token | Size (rem) | Line height | Weight | Usage |
|-------|-----------|-------------|--------|-------|
| `--text-xs` | `0.75` | `1.0` | 400 | Tiny labels, counters |
| `--text-sm` | `0.875` | `1.25` | 400 | Secondary UI labels, captions |
| `--text-base` | `1.0` | `1.5` | 400 | Default UI copy |
| `--text-lg` | `1.125` | `1.55` | 400 | Sidebar headings, list items |
| `--text-xl` | `1.25` | `1.4` | 500 | Panel headings |
| `--text-2xl` | `1.5` | `1.3` | 600 | Modal titles |
| `--text-prose` | `1.125` | `1.8` | 400 | Editor prose (override per user preference — see §3.3) |

### 3.3 User-adjustable prose settings

The editor exposes three user controls that override `--text-prose` and `--font-prose`. These are stored in `~/.booksforge/settings.toml` and applied as inline CSS variables on the `#editor-root` element.

| Setting | Default | Min | Max | Step |
|---------|---------|-----|-----|------|
| Font size | `1.125rem` | `0.875rem` | `1.5rem` | `0.125rem` |
| Line height | `1.8` | `1.5` | `2.2` | `0.1` |
| Line width (max-ch) | `72ch` | `55ch` | `90ch` | `1ch` |

These settings do not affect the export; they are reading preferences only. The export typography comes from the selected template profile.

---

## 4. Spacing scale

All spacing is derived from a base of `4px` (`0.25rem`). Token names are multipliers.

| Token | Value | Common usage |
|-------|-------|-------------|
| `--space-1` | `4px` | Icon padding, tight gaps |
| `--space-2` | `8px` | Button padding (vertical) |
| `--space-3` | `12px` | Input padding |
| `--space-4` | `16px` | Card padding, list item gaps |
| `--space-5` | `20px` | Section gaps inside panels |
| `--space-6` | `24px` | Panel padding |
| `--space-8` | `32px` | Large section gaps |
| `--space-10` | `40px` | Page-level padding |
| `--space-12` | `48px` | Hero / empty-state vertical padding |

---

## 5. Elevation and shadow

| Token | Value (light) | Value (dark) | Usage |
|-------|--------------|--------------|-------|
| `--shadow-sm` | `0 1px 2px rgba(0,0,0,0.06)` | `0 1px 2px rgba(0,0,0,0.4)` | Cards, tooltips |
| `--shadow-md` | `0 4px 12px rgba(0,0,0,0.08)` | `0 4px 12px rgba(0,0,0,0.5)` | Dropdowns, popovers |
| `--shadow-lg` | `0 8px 24px rgba(0,0,0,0.1)` | `0 8px 24px rgba(0,0,0,0.6)` | Modals, dialogs |
| `--shadow-focus` | `0 0 0 3px var(--color-accent-400)` | `0 0 0 3px var(--color-accent-400)` | Keyboard focus ring |

No elevation in focus mode (distraction-free writing). Sidebar and toolbar drop to `opacity: 0` on idle.

---

## 6. Border radius

| Token | Value | Usage |
|-------|-------|-------|
| `--radius-sm` | `4px` | Buttons, inputs, chips |
| `--radius-md` | `8px` | Cards, panels, dropdowns |
| `--radius-lg` | `12px` | Modals, large panels |
| `--radius-full` | `9999px` | Pill badges, toggles |

---

## 7. Motion

Animations respect `prefers-reduced-motion`. When reduced motion is on, durations drop to `0ms` or `1ms`.

| Token | Duration | Easing | Usage |
|-------|----------|--------|-------|
| `--duration-fast` | `100ms` | `ease-out` | Hover states, button feedback |
| `--duration-base` | `150ms` | `ease-in-out` | Panel expand/collapse, fade |
| `--duration-slow` | `250ms` | `ease-in-out` | Modal open, sidebar slide |
| `--duration-agent` | `400ms` | `ease-out` | Agent progress pulse |

---

## 8. Iconography

Icons use [Lucide React](https://lucide.dev/) `^0.447.0` (MIT licence). Size tokens:

| Token | Size | Usage |
|-------|------|-------|
| `--icon-sm` | `14px` | Inline with text labels |
| `--icon-base` | `16px` | Toolbar, sidebar items |
| `--icon-lg` | `20px` | Feature icons, empty states |
| `--icon-xl` | `24px` | Onboarding illustrations |

Icon colour inherits `currentColor` — never hardcode.

---

## 9. Component conventions

### 9.1 Focus management

Every interactive element has a visible `:focus-visible` ring using `--shadow-focus`. `:focus` (not `:focus-visible`) suppresses the ring for mouse users. Tab order follows DOM order; no `tabindex > 0`.

### 9.2 Density

Two density modes, toggled in settings:

| Mode | Body font size | Row height | Description |
|------|---------------|------------|-------------|
| Comfortable (default) | `1rem` | `40px` | For writing sessions |
| Compact | `0.875rem` | `32px` | For outline/binder panels |

### 9.3 Loading states

- **Inline loads** (< 300ms expected): no indicator — avoid flicker.
- **Short waits** (300ms–2s): `aria-busy` spinner at the action site.
- **Agent runs** (2s+): dedicated progress panel per `UI_UX_SPEC.md §6`.
- **Skeleton screens** for the binder and outline view on project open.

### 9.4 Empty states

Every empty-list or first-run state has: an icon, a heading (one line), a body (two lines max), and a primary CTA button. No decorative imagery that increases binary size.

### 9.5 Destructive actions

Destructive actions (delete chapter, remove snapshot) require:

1. A red-tinted button with the error colour.
2. A confirmation dialog with the action name and the item name in the body.
3. No undo available — the dialog must be explicit about this unless a snapshot covers it.

---

## 10. Implementation notes

- Tokens are generated as CSS custom properties in `packages/ui/src/tokens.css` and imported once at the app root.
- Component styles use CSS Modules (`.module.css`). No inline `style` props except for user-adjustable prose settings (§3.3).
- `packages/ui/` exports primitives (Button, Input, Select, Dialog, Toast, Spinner, Badge, Tooltip). Application-specific components live in `apps/desktop/src-ui/src/components/`.
- Colour contrast must meet WCAG 2.1 AA (4.5:1 for normal text, 3:1 for large text and UI components). Run axe-core in Vitest on every component that renders text.
