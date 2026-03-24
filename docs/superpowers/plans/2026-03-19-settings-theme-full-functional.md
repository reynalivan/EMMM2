# Settings Theme Full Functional Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Settings → Appearance theme feature fully functional end-to-end (UI selection, immediate DaisyUI application, system-theme handling, persistence, and reload consistency).

**Architecture:** Keep the current settings data flow (`useSettings` + `save_settings`) and add a dedicated frontend theme runtime layer that translates saved setting values into active DaisyUI themes. Theme selection is owned by `GeneralTab`, theme application is owned by a reusable hook wired at app root, and shared theme rules are centralized in one utility module to avoid duplication. Backend remains the persistence source through existing `AppSettings.theme` support.

**Tech Stack:** React 19, TypeScript, Zustand, TanStack Query, Vitest, Tauri v2, DaisyUI 5

---

## Scope Check

- This request is a single subsystem (Settings theme behavior), so one implementation plan is appropriate.
- No unrelated settings refactor, no backend schema migration, no new feature beyond theme functionality.

## File Responsibility Map

### Frontend Theme Domain (new focused unit)

- Create: `src/features/settings/theme/themeOptions.ts`
  - Responsibility: single source of truth for theme option values/labels and normalization helpers (`system`, `light`, `dark`, `cyberpunk`, `onyx`).

- Create: `src/features/settings/theme/useThemeRuntime.ts`
  - Responsibility: apply active theme to `document.documentElement` (`data-theme`), resolve `system` via `prefers-color-scheme`, and react to OS theme changes only when `system` is selected.

- Create: `src/features/settings/theme/useThemeRuntime.test.tsx`
  - Responsibility: verify theme application behavior and `system` preference listener behavior.

### Settings UI Integration

- Modify: `src/features/settings/tabs/GeneralTab.tsx`
  - Responsibility: make Appearance select truly controlled by persisted settings and DaisyUI `theme-controller` semantics.

- Modify: `src/features/settings/tabs/GeneralTab.test.tsx`
  - Responsibility: verify user theme selection triggers settings persistence with exact payload changes.

### Settings Data Hook Enhancements

- Modify: `src/hooks/useSettings.ts`
  - Responsibility: expose focused `updateTheme` mutation helper (DRY wrapper around `save_settings`) for simple, low-risk theme update calls.

- Modify: `src/hooks/useSettings.test.ts`
  - Responsibility: verify `updateTheme` calls `save_settings` with merged settings and success toast behavior remains intact.

### App Root Wiring

- Modify: `src/App.tsx`
  - Responsibility: initialize runtime theme application once via `useThemeRuntime` so theme is applied globally and consistently after app startup.

---

## Chunk 1: Worktree + Theme Contract Tests

### Task 1: Create isolated worktree and baseline

**Files:**

- Modify: none
- Test: none

- [ ] **Step 1: Create dedicated worktree**

Run:

```bash
git worktree add ../settings-theme-full-functional -b feat/settings-theme-full-functional
```

Expected: new worktree created and branch checked out.

- [ ] **Step 2: Verify clean branch state**

Run:

```bash
git status
```

Expected: `On branch feat/settings-theme-full-functional` and clean working tree.

- [ ] **Step 3: Capture baseline GeneralTab tests**

Run:

```bash
pnpm vitest run src/features/settings/tabs/GeneralTab.test.tsx
```

Expected: current tests pass (or pre-existing failures documented before proceeding).

- [ ] **Step 4: Capture baseline useSettings tests**

Run:

```bash
pnpm vitest run src/hooks/useSettings.test.ts
```

Expected: current tests pass (or pre-existing failures documented before proceeding).

- [ ] **Step 5: Commit baseline notes only if files changed**

Run only if needed:

```bash
git add <baseline-note-files>
git commit -m "chore(settings): capture theme baseline verification"
```

### Task 2: Add failing tests for full theme functionality (TDD)

**Files:**

- Create: `src/features/settings/theme/useThemeRuntime.test.tsx`
- Modify: `src/features/settings/tabs/GeneralTab.test.tsx`
- Modify: `src/hooks/useSettings.test.ts`

- [ ] **Step 1: Add failing test for persisted theme application at app runtime**

```tsx
it('applies data-theme from settings.theme on mount', () => {
  // mock settings.theme = "cyberpunk"
  // render hook/component using useThemeRuntime
  // expect(document.documentElement.dataset.theme).toBe("cyberpunk")
});
```

- [ ] **Step 2: Add failing test for system theme resolution**

```tsx
it('maps settings.theme=system to dark or light from matchMedia', () => {
  // mock prefers-color-scheme: dark
  // expect applied data-theme to be "dark"
});
```

- [ ] **Step 3: Add failing test for GeneralTab theme change persistence**

```tsx
it('updates only theme via updateTheme when user selects a new option', async () => {
  // user selects "light"
  // expect updateTheme("light") called once
});
```

- [ ] **Step 4: Add failing test for useSettings updateTheme mutation contract**

```tsx
it('updateTheme merges with current settings and invokes save_settings', async () => {
  // expect invoke('save_settings', { settings: { ...existing, theme: 'onyx' } })
});
```

- [ ] **Step 5: Run targeted tests to confirm failures**

Run:

```bash
pnpm vitest run src/features/settings/theme/useThemeRuntime.test.tsx src/features/settings/tabs/GeneralTab.test.tsx src/hooks/useSettings.test.ts
```

Expected: FAIL with missing hook/helper wiring and missing theme update behavior.

- [ ] **Step 6: Commit tests-only checkpoint**

```bash
git add src/features/settings/theme/useThemeRuntime.test.tsx src/features/settings/tabs/GeneralTab.test.tsx src/hooks/useSettings.test.ts
git commit -m "test(settings): define full theme behavior contract"
```

---

## Chunk 2: Implement Theme Runtime + Settings Integration

### Task 3: Implement shared theme options and runtime application layer

**Files:**

- Create: `src/features/settings/theme/themeOptions.ts`
- Create: `src/features/settings/theme/useThemeRuntime.ts`
- Test: `src/features/settings/theme/useThemeRuntime.test.tsx`

- [ ] **Step 1: Implement theme constants and helpers in one place (DRY)**

```ts
export const THEME_OPTIONS = [
  { value: 'system', label: 'System Default' },
  { value: 'dark', label: 'Dark' },
  { value: 'light', label: 'Light' },
  { value: 'cyberpunk', label: 'Cyberpunk' },
  { value: 'onyx', label: 'Onyx (EMMM)' },
] as const;

export type ThemeSetting = (typeof THEME_OPTIONS)[number]['value'];

export function resolveTheme(
  setting: ThemeSetting,
  prefersDark: boolean,
): 'dark' | 'light' | 'cyberpunk' | 'onyx' {
  if (setting === 'system') return prefersDark ? 'dark' : 'light';
  return setting;
}
```

- [ ] **Step 2: Implement `useThemeRuntime` to set `data-theme` on `<html>`**

```ts
useEffect(() => {
  const media = window.matchMedia('(prefers-color-scheme: dark)');
  const apply = () => {
    const active = resolveTheme(themeSetting, media.matches);
    document.documentElement.setAttribute('data-theme', active);
  };
  apply();
  if (themeSetting !== 'system') return;
  media.addEventListener('change', apply);
  return () => media.removeEventListener('change', apply);
}, [themeSetting]);
```

- [ ] **Step 3: Re-run runtime tests**

Run:

```bash
pnpm vitest run src/features/settings/theme/useThemeRuntime.test.tsx
```

Expected: PASS.

- [ ] **Step 4: Commit runtime layer**

```bash
git add src/features/settings/theme/themeOptions.ts src/features/settings/theme/useThemeRuntime.ts src/features/settings/theme/useThemeRuntime.test.tsx
git commit -m "feat(settings): add daisyui theme runtime layer"
```

### Task 4: Wire settings hook and GeneralTab to real persistence

**Files:**

- Modify: `src/hooks/useSettings.ts`
- Modify: `src/hooks/useSettings.test.ts`
- Modify: `src/features/settings/tabs/GeneralTab.tsx`
- Modify: `src/features/settings/tabs/GeneralTab.test.tsx`
- Test: `src/features/settings/tabs/GeneralTab.test.tsx`, `src/hooks/useSettings.test.ts`

- [ ] **Step 1: Add `updateTheme` helper in `useSettings`**

```ts
const updateThemeMutation = useMutation({
  mutationFn: async (theme: ThemeSetting) => {
    if (!settingsQuery.data) throw new Error('Settings not loaded');
    const newSettings = { ...settingsQuery.data, theme };
    return invoke('save_settings', { settings: newSettings });
  },
  onSuccess: () => queryClient.invalidateQueries({ queryKey: settingsKeys.all }),
});
```

- [ ] **Step 2: Replace static Theme select in `GeneralTab` with controlled value + handler**

```tsx
const { settings, updateTheme } = useSettings();

<select
  className="select select-bordered w-full theme-controller"
  value={settings?.theme ?? 'dark'}
  onChange={(e) => updateTheme.mutate(e.target.value as ThemeSetting)}
>
  {THEME_OPTIONS.map((option) => (
    <option key={option.value} value={option.value}>
      {option.label}
    </option>
  ))}
</select>;
```

- [ ] **Step 3: Ensure graceful UI state during save**

```tsx
disabled={updateTheme.isPending || !settings}
```

- [ ] **Step 4: Run targeted tests**

Run:

```bash
pnpm vitest run src/features/settings/tabs/GeneralTab.test.tsx src/hooks/useSettings.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit settings integration**

```bash
git add src/hooks/useSettings.ts src/hooks/useSettings.test.ts src/features/settings/tabs/GeneralTab.tsx src/features/settings/tabs/GeneralTab.test.tsx
git commit -m "feat(settings): persist and control theme selection"
```

---

## Chunk 3: App Wiring + End-to-End Verification

### Task 5: Apply theme globally from app root

**Files:**

- Modify: `src/App.tsx`
- Test: `src/features/settings/theme/useThemeRuntime.test.tsx` (already covers behavior)

- [ ] **Step 1: Wire `useThemeRuntime` in `App` root component**

```tsx
export default function App() {
  useThemeRuntime();
  return (...);
}
```

- [ ] **Step 2: Verify no regression in app shell test**

Run:

```bash
pnpm vitest run src/App.test.tsx
```

Expected: PASS.

- [ ] **Step 3: Commit root wiring**

```bash
git add src/App.tsx
git commit -m "feat(settings): apply active theme globally"
```

### Task 6: Full verification for theme feature and docs note

**Files:**

- Modify: `docs/superpowers/plans/2026-03-19-settings-theme-full-functional.md` (checkbox updates only, optional)

- [ ] **Step 1: Run complete targeted frontend verification**

Run:

```bash
pnpm vitest run src/features/settings/tabs/GeneralTab.test.tsx src/features/settings/theme/useThemeRuntime.test.tsx src/hooks/useSettings.test.ts src/App.test.tsx
```

Expected: PASS.

- [ ] **Step 2: Run lint check for touched TS/TSX files**

Run:

```bash
pnpm lint
```

Expected: PASS or only pre-existing unrelated warnings documented.

- [ ] **Step 3: Manual smoke test in Tauri app**

Run:

```bash
pnpm tauri dev
```

Manual expected behavior:

- Open Settings → General → Theme.
- Switch among `dark`, `light`, `cyberpunk`, `onyx`: UI updates immediately.
- Select `system`: app follows OS theme.
- Restart app: previously selected setting persists and reapplies.

- [ ] **Step 4: Final implementation commit (if anything remains)**

```bash
git add <remaining intended files>
git commit -m "chore(settings): finalize full theme functionality verification"
```

---

## Acceptance Criteria

- Theme dropdown in Settings is no longer static and updates persisted `settings.theme`.
- Active DaisyUI theme is applied globally via `data-theme` in runtime.
- `system` setting follows OS light/dark preference and reacts to preference changes while app is open.
- Theme choice persists across app reload/restart.
- Targeted tests for runtime hook, settings mutation helper, and GeneralTab interactions pass.

## Non-Goals

- No redesign of Settings navigation/layout.
- No backend schema changes.
- No language/i18n enhancements in this task.

## Notes for Implementers

- Keep logic centralized in `src/features/settings/theme/*`; do not duplicate theme lists in multiple files.
- Keep `GeneralTab` focused on UI wiring only.
- Preserve existing auto-close launcher behavior and tests.
- Follow commit granularity above (one logical slice per commit).
