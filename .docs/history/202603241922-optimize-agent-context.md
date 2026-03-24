# Optimize AGENT.md Context Efficiency

## Context

The previous `AGENT.md` was verbose and contained redundant rules (especially for i18n and logging), which consumed unnecessary AI context tokens and reduced parsing efficiency.

## Changes

- **Consolidation**: Merged repetitive i18n and logging rules into single, high-impact points.
- **Directive Mode**: Refactored descriptions into concise technical sentences.
- **Information Architecture**: Grouped axioms, compliance, workflow, and architecture for better AI navigation.
- **Final Checks**: Restored specifically requested "Final Compliance Check" list with mandatory checkboxes.
- **Efficiency**: Reduced file length by approximately 35% while maintaining 100% rule coverage.

## Impacted Files

- `AGENT.md` (modified)

## Goal

Improve AI agent understanding and efficiency during complex sessions.

## Impact

- Lower context usage per session.
- Faster AI "onboarding" to project-specific rules.
- No breaking changes.
