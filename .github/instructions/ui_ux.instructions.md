---
trigger: model_decision
description: UI/UX Rules - DaisyUI, layout, and motion.
---

- Style: daisyUI 5 + Tailwind 4 + Lucide. Semantic colors only.
- Themes: Dark default. Support Light. CSS Grid shell.
- Virt: Virtualize lists/grids > 50 items.
- Motion: 150-250ms subtle motion (motion.dev). Skeletons mandatory.
- Hygiene: Game/Mode switch MUST clear grid Selection/Path.
- Banner: `runtime_status.txt` MUST be ≤ 10 lines and ≤ 4KB.
- Keybind: Mode/Collection toggle MUST trigger 3DMigoto Reload (F10).
- Verify: mcp_daisyui_get_component before building.
