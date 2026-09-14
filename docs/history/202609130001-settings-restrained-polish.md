# Restrained Settings polish

## Context

Settings used repeated coloured action cards and exposed implementation details that do not help users configure the app.

## Changes

- Reworked the Settings shell, General, and Maintenance tabs into a quieter section-and-divider layout.
- Reduced General to user-facing controls: theme selector, language, launch behaviour, and app version.
- Removed the visible Tauri, database, engine, and theme-information details.
- Reserved strong colour for the destructive reset action; other maintenance actions use compact neutral controls.

## Impacted Files

- `src/pages/settings/SettingsPage.tsx`
- `src/pages/settings/components/tabs/GeneralTab.tsx`
- `src/pages/settings/components/tabs/GeneralTab.test.tsx`
- `src/pages/settings/components/tabs/MaintenanceTab.tsx`

## Goal

Make Settings easier to scan while preserving its existing controls and behaviour.

## Impact

Custom themes remain selectable, but theme import/export/delete controls are no longer shown in General.
