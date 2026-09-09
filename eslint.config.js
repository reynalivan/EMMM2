import js from '@eslint/js';
import globals from 'globals';
import reactHooks from 'eslint-plugin-react-hooks';
import reactRefresh from 'eslint-plugin-react-refresh';
import tseslint from 'typescript-eslint';
import prettier from 'eslint-plugin-prettier';
import prettierConfig from 'eslint-config-prettier';

/**
 * Architecture rules for the mods runtime.
 *
 * These replace the filesystem-walking `*.audit.test.ts` suites: same bans, but
 * enforced in the editor at edit time instead of at test time, and expressed as
 * selectors rather than substring greps (so they cannot match a comment or a
 * string literal by accident).
 */

/** Directories that own the mods runtime. */
const MODS_RUNTIME = [
  'src/widgets/object-sidebar/**',
  'src/widgets/mod-explorer/**',
  'src/widgets/mod-preview/**',
  'src/features/mod-runtime/**',
  'src/features/file-watcher/**',
  'src/features/workspace-runtime/**',
  'src/features/import-batches/**',
  'src/features/match-wizard/**',
  'src/features/randomizer/**',
  'src/hooks/**',
];

/** Runtime directories that consume the shared actions rather than defining them. */
const MODS_RUNTIME_CONSUMERS = [
  'src/widgets/object-sidebar/**',
  'src/widgets/mod-explorer/**',
  'src/widgets/mod-preview/**',
  'src/features/mod-runtime/**',
  'src/features/workspace-runtime/**',
];

/** Ban a bare `name(...)` call. */
const bannedCall = (name, message) => ({
  selector: `CallExpression[callee.type="Identifier"][callee.name="${name}"]`,
  message,
});

/** Ban a `object.property(...)` call. */
const bannedMethod = (object, property, message) => ({
  selector: `CallExpression[callee.object.name="${object}"][callee.property.name="${property}"]`,
  message,
});

/** Ban any mention of an identifier, called or not. */
const bannedIdentifier = (name, message) => ({
  selector: `Identifier[name="${name}"]`,
  message,
});

export default tseslint.config(
  {
    ignores: [
      'dist',
      'target',
      'src-tauri',
      'coverage',
      // Ignore specta generated bindings
      'src/shared/api/tauri/bindings.gen.ts',
      '.agent',
      '.history',
      '.vscode',
      '.opencode',
      '.codebuddy',
    ],
  },
  {
    extends: [js.configs.recommended, ...tseslint.configs.recommended, prettierConfig],
    files: ['**/*.{ts,tsx}'],
    languageOptions: {
      ecmaVersion: 2020,
      globals: globals.browser,
    },
    plugins: {
      'react-hooks': reactHooks,
      'react-refresh': reactRefresh,
      prettier: prettier,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      'react-hooks/set-state-in-effect': 'off',
      'react-refresh/only-export-components': ['warn', { allowConstantExport: true }],
      'prettier/prettier': 'warn',
      '@typescript-eslint/no-unused-vars': ['warn', { argsIgnorePattern: '^_' }],
      'max-lines': ['warn', { max: 350, skipBlankLines: true, skipComments: true }],
    },
  },
  {
    files: ['src/test-utils.tsx', 'src/setupTests.ts'],
    rules: {
      'react-refresh/only-export-components': 'off',
    },
  },

  // --- Mods runtime architecture ---

  // runtime-sync is the lower layer: importing workspace-runtime would close a cycle.
  {
    files: ['src/features/runtime-sync/**/*.{ts,tsx}'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          patterns: [
            {
              group: ['**/workspace-runtime/**', '**/workspace-runtime'],
              message:
                'runtime-sync is the lower layer; importing workspace-runtime closes a cycle. Put shared contracts in src/core/lib/runtimeEffects.ts.',
            },
          ],
        },
      ],
    },
  },

  // The descriptor contract modules import downward only.
  {
    files: [
      'src/features/workspace-runtime/optimistic/descriptor.ts',
      'src/features/workspace-runtime/optimistic/descriptorBuilders.ts',
    ],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          patterns: [
            {
              group: ['**/runtime-sync/**', '**/runtime-sync'],
              message:
                'Descriptor contracts must not import runtime-sync; they are consumed by it, not the other way round.',
            },
          ],
        },
      ],
    },
  },

  // runtimeEffects is the shared leaf contract: no feature may leak into it.
  {
    files: ['src/shared/lib/runtimeEffects.ts'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          patterns: [
            {
              group: ['**/features/**'],
              message: 'runtimeEffects.ts is a leaf contract module and must not import features.',
            },
          ],
        },
      ],
    },
  },

  // Runtime-owning directories: refresh and selection go through the central helpers.
  {
    files: MODS_RUNTIME,
    rules: {
      'no-restricted-imports': [
        'error',
        {
          patterns: [
            {
              group: ['**/hooks/useFolders', '**/hooks/useObjects'],
              message:
                'Do not reach through the public useFolders/useObjects barrels for internal runtime helpers; import the helper module directly.',
            },
          ],
        },
      ],
      'no-restricted-syntax': [
        'error',
        bannedCall(
          'publishRuntimeEvents',
          'Publish through a runtime effect descriptor instead of calling publishRuntimeEvents directly.',
        ),
        bannedCall(
          'refreshObjectListQueries',
          'refreshObjectListQueries is internal to the refresh infra; publish a descriptor instead.',
        ),
        bannedCall(
          'refreshRuntimeQueries',
          'refreshRuntimeQueries is internal to the refresh infra; publish a descriptor instead.',
        ),
        bannedCall('focusWorkspaceObject', 'Removed runtimeSelection helper.'),
        bannedCall('syncExplorerToObjectRoot', 'Removed runtimeSelection helper.'),
        bannedCall('applyWorkspaceExplorerLocation', 'Removed runtimeSelection helper.'),
        bannedCall('clearWorkspaceSelection', 'Removed runtimeSelection helper.'),
        bannedCall('usePreviewPanelActions', 'Removed preview wrapper.'),
        bannedCall('useSelectedModPath', 'Removed selection helper.'),
      ],
    },
  },

  // Consumer surfaces: no legacy query helpers, no direct dialog or IPC shortcuts.
  {
    files: MODS_RUNTIME_CONSUMERS,
    rules: {
      'no-restricted-syntax': [
        'error',
        bannedCall(
          'useObjects',
          'Read objects from the workspace view model, not the legacy hook.',
        ),
        bannedCall(
          'useModFolders',
          'Read folders from the workspace view model, not the legacy hook.',
        ),
        bannedMethod(
          'commands',
          'getObjectsCmd',
          'Consumers read objects from the workspace view model, not directly over IPC.',
        ),
        bannedMethod(
          'commands',
          'listObjectModPaths',
          'Removed IPC command; resolve mod paths through the workspace view model.',
        ),
        bannedMethod(
          'commands',
          'listModFolders',
          'Removed IPC command; read folders from the workspace view model.',
        ),
        bannedCall(
          'openConflictDialog',
          'Open runtime dialogs through the shared runtime actions.',
        ),
        bannedCall(
          'openDuplicateConflictDialog',
          'Open runtime dialogs through the shared runtime actions.',
        ),
        bannedCall(
          'openFileInUseDialog',
          'Open runtime dialogs through the shared runtime actions.',
        ),
        bannedCall(
          'useObjectSelectionRepair',
          'Stale selection repair belongs to the backend reconcile pass, not a frontend probe.',
        ),
        bannedIdentifier(
          'requestRepairSync',
          'Stale selection repair belongs to the backend reconcile pass, not a frontend probe.',
        ),
      ],
    },
  },

  // importPipeline preflights a user-supplied path before import; that probe is allowed.
  {
  files: ['src/widgets/object-sidebar/utils/importPipeline.ts'],
    rules: { 'no-restricted-syntax': 'off' },
  },

  // Whole frontend: removed commands stay removed, refresh stays centralised.
  {
    files: ['src/**/*.{ts,tsx}'],
    ignores: ['src/shared/api/tauri/bindings.ts', 'src/setupTests.ts'],
    rules: {
      'no-restricted-properties': [
        'error',
        { object: 'commands', property: 'toggleMod', message: 'Removed switch compat command.' },
        {
          object: 'commands',
          property: 'enableOnlyThis',
          message: 'Removed switch compat command.',
        },
        {
          object: 'commands',
          property: 'checkDuplicateEnabled',
          message: 'Removed switch compat command.',
        },
        {
          object: 'commands',
          property: 'undoCollection',
          message: 'Removed command; use the collections apply/undo flow.',
        },
      ],
    },
  },

  // invalidateQueries is the escape hatch the descriptor pipeline exists to replace.
  {
    files: ['src/**/*.{ts,tsx}'],
    ignores: [
      'src/shared/lib/queryRefresh.ts',
      'src/features/workspace-runtime/optimistic/applyOptimisticEffects.ts',
      '**/*.test.{ts,tsx}',
    ],
    rules: {
      'no-restricted-syntax': [
        'error',
        {
          selector: 'CallExpression[callee.property.name="invalidateQueries"]',
          message:
            'Raw invalidateQueries bypasses the refresh pipeline. Publish a runtime effect descriptor instead.',
        },
        {
          selector: 'Property[key.name="refreshEvents"] > ArrayExpression',
          message:
            'Do not build literal refresh event arrays in feature code; use a builder from descriptorBuilders.ts.',
        },
      ],
    },
  },

  // Menu policy is pure: it decides what is available, it does not act.
  {
    files: [
      'src/hooks/useModContextMenuItems.ts',
      'src/features/mod-runtime/actions/modContextMenuPolicy.ts',
    ],
    rules: {
      'no-restricted-syntax': [
        'error',
        {
          selector: 'MemberExpression[object.name="commands"]',
          message: 'Menu policy decides availability only; perform IPC in the action hooks.',
        },
        {
          selector: 'MemberExpression[object.name="navigator"][property.name="clipboard"]',
          message: 'Menu policy must stay free of imperative side effects.',
        },
        bannedCall('openDialog', 'Menu policy must stay free of imperative side effects.'),
        bannedCall('alert', 'Menu policy must stay free of imperative side effects.'),
        bannedMethod('console', 'error', 'Menu policy must stay free of imperative side effects.'),
        bannedCall(
          'publishRuntimeDescriptor',
          'Menu policy must stay free of imperative side effects.',
        ),
      ],
    },
  },

  // Surface components render; they do not wire events or publish runtime effects.
  {
    files: [
      'src/widgets/mod-preview/PreviewPanel.tsx',
      'src/widgets/mod-explorer/FolderGrid.tsx',
      'src/widgets/object-sidebar/ObjectList.tsx',
    ],
    rules: {
      'no-restricted-syntax': [
        'error',
        bannedMethod(
          'window',
          'dispatchEvent',
          'Surface components must not wire events directly.',
        ),
        bannedMethod(
          'window',
          'addEventListener',
          'Surface components must not wire events directly.',
        ),
        bannedCall(
          'publishRuntimeDescriptor',
          'Surface components must not publish runtime effects; do it in the action hooks.',
        ),
      ],
    },
  },

  // Folder-grid dialogs collect input; the caller publishes and reports errors.
  {
    files: [
      'src/widgets/object-sidebar/modals/MoveToObjectDialog.tsx',
      'src/widgets/mod-explorer/modals/IgnoreManagementModal.tsx',
    ],
    rules: {
      'no-restricted-syntax': [
        'error',
        bannedCall(
          'publishRuntimeDescriptor',
          'Dialogs collect input; the calling action publishes the runtime effect.',
        ),
        bannedMethod('console', 'error', 'Surface errors through the toast pipeline, not console.'),
      ],
    },
  },

  // --- Scanner / import boundary ---

  // Match analysis is user-initiated. Passive runtime paths only trigger Disk Reconcile,
  // so they must never start or refresh a Match Wizard workflow.
  {
    files: [
      'src/app/entrypoint/App.tsx',
      'src/shared/ui/components/layout/**/*.{ts,tsx}',
      'src/features/file-watcher/**/*.{ts,tsx}',
      'src/pages/onboarding/**/*.{ts,tsx}',
      'src/features/workspace-runtime/**/*.{ts,tsx}',
    ],
    rules: {
      'no-restricted-syntax': [
        'error',
        ...[
          'createImportBatch',
          'analyzeImportBatch',
          'refreshImportItemSuggestions',
          'previewObjectClassificationBatch',
        ].map((name) =>
          bannedMethod(
            'commands',
            name,
            'Passive runtime paths trigger Disk Reconcile only; Match Wizard is user-initiated.',
          ),
        ),
      ],
    },
  },
);
