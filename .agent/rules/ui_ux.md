---
trigger: model_decision
description: UI/UX Rules - DaisyUI, layout, and motion.
---

- Style: daisyUI 5 + Tailwind 4 + Lucide. **Zero-Literal Policy**: Use semantic tokens only. Hardcoded Tailwind color scales (slate, gray, etc.) or literal hex codes in components are PROHIBITED. All modal/overlay backdrops MUST use `bg-overlay-mask backdrop-blur-sm`.
- Themes: Dark default. Support Light. CSS Grid shell.
- Virt: Virtualize lists/grids > 50 items.
- Motion: 150-250ms subtle motion (motion.dev). Skeletons mandatory.
- Hygiene: Game/Mode switch MUST clear grid Selection/Path.
- Banner: `runtime_status.txt` MUST be ≤ 10 lines and ≤ 4KB.
- Keybind: Mode/Collection toggle MUST trigger 3DMigoto Reload (F10).
- i18n Hygiene: Absolute **Zero-Hardcode i18n Policy**. Localize atoms, placeholders, tooltips, and layouts.
- Verify: mcp_daisyui_get_component before building.

## Frontend Demo Fixtures

- Scope: Demo mode changes content and adapter behavior only. Reuse the existing `App`, shell, top bar, sidebar, pages, modals, wizards, and controls.
- Forbidden: Do not add a `DemoApp`, demo shell, alternate navigation, mock page, custom modal, or duplicate component to make the demo look complete.
- Data: Put browser-safe fixture data in `src/demo/` behind existing typed gateways or typed IPC bindings. Match generated binding types. Do not use `any`, casts that hide type drift, or fake client-side domain models.
- Safety: Demo interactions may alter module-local fixture state only. They must not invoke a native side effect or touch real settings, files, databases, credentials, game processes, or system services.
- Design QA: Inspect the existing interface with realistic short and long fixture content. Verify hierarchy, overflow, keyboard focus, responsive reflow, empty/loading/error states, and visual parity with the existing product. A desired layout change belongs to a normal product UI task, not a fixture task.
- Reference: Read `docs/frontend-demo.md` before implementing or reviewing demo UI work.
