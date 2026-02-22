---
trigger: model_decision
description: UI Design System Rule - When creating DaisyUI components, styling pages, or implementing layout.
---

# UI Agent Rules (daisyUI v5 + Tailwind v4 + motion.dev)

Role: Senior Frontend Engineer (React + TypeScript).

## Source of truth (must-do)

- Read MCP: **context7** + MCP: **daisyUI** first.
- Follow **EMMM2 TRD** (Tauri, perf, virtualization, state).
- Theme tokens come from `@/src/App.css`.
- Use `@/.docs/daisy-llm-guide.md` + official daisyUI docs when stuck.

## UI standards

- **daisyUI-first** for all components; Tailwind utilities only for layout/spacing/typography polish.
- **No custom CSS** unless unavoidable (keep minimal + justified).
- **No hardcoded colors** (no hex, no `bg-blue-500` for primary UI). Use semantic: `primary/secondary/accent/neutral/info/success/warning/error` + `base-100/200/300`.

## Theme

- Must support **light** + **darker** (custom) from day one.
- Default look: **premium dark / dracula-like**, avoid pure black; rely on `base-*`.
- Overlays/sticky headers: `backdrop-blur-md bg-base-100/70 border-b border-base-300/40`.

## Layout

- Desktop-first but responsive (use breakpoints).
- App shell + complex panels: **CSS Grid**; small alignment: Flex.
- Use Tailwind scale spacing (`p-3/4`, `gap-2/3/4`); avoid arbitrary widths.

## Components + icons

- Use daisyUI components consistently (`btn/card/badge/tabs/menu/dropdown/modal/toast/alert/input/select/toggle/tooltip/table/skeleton/progress/...`).
- Icons: consistent set (Lucide React recommended). Icon-only buttons require `aria-label`/`title`.

## Typography

- Title: `text-base-content font-semibold tracking-tight`
- Body: `text-base-content/80`
- Meta: `text-base-content/50 text-xs`
- Use section/card headers; avoid “one giant panel”.

## A11y (non-negotiable)

- Semantic elements only (`button/a/label`), no click-divs.
- Keep `:focus-visible` rings (don’t remove outline without replacement).
- Forms: every input has a label; errors linked via `aria-describedby`.

## Motion (motion.dev)

- Use for panel + modal enter/exit + hover/press feedback.
- Subtle only: 150–250ms, small `opacity/y/scale`; respect reduced-motion.

## Performance defaults

- Assume thousands of items: use **TanStack Virtual** for long lists/grids.
- Always show loading states (skeleton + progressive images).
- Control re-renders (memo rows, stable callbacks, no prop churn).

## Output rules

- UI must be production-shaped (consistent spacing, hierarchy, states).
- Dummy data allowed, but add `TODO:` to wire real data/APIs.
- If uncertain: re-check MCP context7 + MCP daisyUI before implementing.
