# Clarify native Windows verification

## Context

User requested consistent demo, agent rules and skills that route application
testing to the native Windows app instead of browser demo QA.

## Changes

- Make native WDIO/tauri-driver/WebView2 the default for desktop flows;
  browser fixture review requires an explicit user request.
- Add the missing referenced E2E skill and correct spec location/workflow.
- Preserve fresh-build isolation, real watchers/readiness, visible assertions,
  actual process restarts and owned teardown/fixture cleanup.
- Define blocked-native reporting and documentation-only validation scope.

## Impacted Files

- `AGENT.md`; `docs/frontend-demo.md`.
- `.agent/rules/{project_core,dev_ops,ui_ux}.md`.
- `.agent/workflows/e2e-testing.md`.
- `.agent/skills/{e2e-automation,code-reviewer,tauri-command}/SKILL.md`.
- This history file.

## Goal and Impact

Future app verification uses the existing isolated Windows harness. Demo remains
available for explicitly requested fixture review. No application code, harness,
global user configuration or dependencies changed; no app test was needed for
these instruction-only edits.

## Validation

New skill structure validator, documentation formatting, native routing/link
references and `git diff --check` passed. No application tests run: prose only.
