---
trigger: always_on
description: Core Context & Zero Tolerance - Essential constraints for intelligence, logic integrity, and tool usage.
---

- No Guessing: Never invent APIs/paths/schemas. Unclear? Ask.
- Context: Read dependencies/types before editing.
- Single Truth: No duplicate logic. DRY/Reuse only.
- Scope: Only change requested. No unrelated refactors.
- MCPs: context7 (Docs), daisyui (UI), narsil-mcp (Flow/Security). Trace logic FIRST.
- History: Read 3-4 latest files from `.docs/history/` to understand recent implementation context.
- Memory: Search supermemory for past decisions before asking.
- Safe Edit: No placeholders/skips (//...). Output full file or valid UD.
- Check: Verify: no assumptions, used MCPs, no unused code, match context, **theme-aware?**
