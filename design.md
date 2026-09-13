# Design — EMMM

## Genre

Technical, modern-minimal workspace application with **QuickLiquid Liquid Chrome**.
The liquid effect is a material language for navigation and high-value controls,
not a replacement for dense, readable data surfaces.

## App-page structure

TopBar owns page identity. Workspace pages use a compact context bar followed by
either a wide data surface, a constrained form surface, or a dense tool
workspace. App pages share the **Workbench** macrostructure: stable global
chrome, compact contextual controls, then task-focused content.

## Shell overlay and safe inset

TopBar and contextual action bars are visual chrome overlays. Scrollable page
content must move beneath their Frosted Liquid backdrop so the material can
blur real content, not a solid spacer. The first readable content position must
still clear the chrome.

Every independent scroll owner, including page frames, sidebars, object lists,
folder grids, preview panes, and browser toolbars, must consume a shared safe
inset token. Derive that token from the measured overlay chrome height and a
small content gap. Use the existing shell tokens such as
`--workspace-topbar-height`, `--workspace-panel-content-inset`, and
pane-specific chrome-height variables. Do not add page-local `pt-16`, magic
margins, or viewport-width assumptions to compensate for overlay chrome.

When a pane has its own sticky action bar, its content inset must include both
the global TopBar and that bar's actual height. Measure responsive chrome where
its height can change, then update the pane token. Overlay menus and modal
layers must remain above their scroll owner and must never be clipped by it.

## Theme

Use the existing semantic DaisyUI tokens in `src/app/entrypoint/App.css`.
Primary blue is the only non-status accent; it may appear in liquid rim lighting
and the active control state, but not as a pervasive glow. Success, warning,
and error communicate state or destructive actions only.

The light and onyx themes must use the same semantic hue families. Liquid
material adapts its tint and shadow to the active theme; it must not introduce a
separate palette or hard-coded component colours.

## Typography and spacing

Use the existing sans and mono roles. Keep headings roman, use the existing
4-point Tailwind scale, and constrain form content to `max-w-3xl` and data
content to `max-w-7xl`. Text and data always sit above the liquid layer with
full contrast; glass never reduces the legibility of labels, tables, or inputs.

## Surface and actions

### Base surfaces

Data tables, lists, forms, editor panes, cards, and sidebars use neutral,
mostly opaque surfaces with a hairline border and the shared radius token. Do
not apply QuickLiquid to repeated rows, virtualized content, or every card.

### Liquid Chrome

Use `quick-liquid/react` through shared internal primitives rather than direct
imports in page components. The allowed material roles are:

- `liquid-nav` — `regular`: TopBar, app menu, and command-like popovers.
- `liquid-control` — `thin`: game selector, launch control, compact floating
  actions, and selected bulk actions.
- `liquid-indicator` — `clear`: active tab/pill indicators and small status
  affordances.
- `liquid-overlay` — `thick` or `ultra`: only a modal or transient sheet whose
  content remains readable at all supported viewport sizes.

Liquid surfaces need a deliberate, contrast-bearing backdrop to reveal the
refractive bezel. On a uniform workspace background, retain the subtle
blur/tint fallback rather than adding decorative gradients behind every page.
Use faint rim lighting and chromatic edges only at the bezel; never use a
uniform bright border, large halo, or opaque "milky" glass.

Primary buttons advance the main task; outline and ghost buttons are secondary;
error styling is destructive only. Liquid is material, not an additional action
priority.

## QuickLiquid implementation guardrails

- Pin and wrap the dependency behind shared `LiquidSurface` and
  `LiquidTabIndicator` components. Existing `.glass-surface` consumers remain
  unchanged until explicitly migrated.
- Prefer `quality: 'high'` for the small global chrome set. Use `medium` or
  `low` only after measuring dense or background UI. Never initialize an engine
  per list row.
- Keep `dynamicLighting`, pointer tracking, parallax, droplet merges, and
  liquid press off by default. Enable only where the interaction communicates a
  real action, such as the primary launch control.
- Do not place `isolation`, `filter`, `mask`, `mix-blend-mode`, or reduced
  `opacity` on a liquid host or an ancestor that supplies its backdrop. Avoid
  explicit stacking changes on QuickLiquid's internal lens layer.
- Native Tauri browser webviews render above DOM content. Apply liquid only to
  the browser's DOM chrome; never expect it to refract the embedded webview.
- Keep an ordinary CSS blur/tint fallback for unsupported rendering paths and a
  user-visible setting to disable liquid effects.

## Custom theme background

Built-in themes remain simple, opaque colour fields. A custom theme must define
`config.background` alongside its required `config.liquid` block. Its `kind` is
`solid`, `gradient`, or `image`; `value` is a CSS colour/gradient for the first
two modes and an absolute local file path for image mode. `dim_opacity` (0–1)
controls the base-colour veil above the global shell background so foreground
text remains readable. The image stays behind the workspace chrome; data
surfaces, forms, tables, and native browser webviews remain opaque.

## Motion and accessibility

Use 150–160 ms transitions for colour, border, opacity, and at most a 1 px
interactive-surface lift. Never use `transition-all` or hover scale. Liquid
press must be subtle and only on controls; it must never alter layout.

Respect reduced motion: stop cursor-driven lighting, parallax, spring gestures,
and continuous movement; use a static tint or an opacity-only transition of
150 ms or less. Every hover affordance needs a keyboard focus and touch-visible
equivalent. Focus rings remain immediate, high-contrast, and above the liquid
layer.

## Page allowances

- **TopBar:** primary liquid surface; app menu, game selector, primary launch,
  and context actions may use the shared material roles.
- **Dashboard:** Quick Play and selected navigation controls may use liquid;
  metrics, charts, recent-mod rows, and keybinding tables remain opaque.
- **Mods Manager:** use liquid for toolbar, selection controls, and a sticky
  preview header only; object lists, folder grids, and editor sections remain
  dense tool surfaces.
- **Collections, Mod Inbox, Storage Optimizer:** use liquid for active tab
  indicators, scan/apply controls, and transient action bars; list/report rows
  remain opaque.
- **Discover and Downloads:** browser tab indicator, toolbar, find panel, and
  download controls may use liquid. Browser content itself remains functional
  native-webview chrome.
- **Settings:** use liquid for the selected-tab indicator and a contained
  appearance preview only. Settings content remains section lists and dividers,
  not card stacks.
- **Onboarding:** the existing aurora and demo strip may be refined as a later,
  isolated use case after the global chrome passes visual QA.
