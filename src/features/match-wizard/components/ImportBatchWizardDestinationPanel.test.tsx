import { render, screen } from '@testing-library/react';
import type { ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';
import type {
  DestinationSuggestion,
  ImportBatch,
  ImportItem,
  ObjectSummary,
} from '../../../shared/api/tauri/bindings.gen';
import { ImportBatchWizardDestinationPanel } from './ImportBatchWizardDestinationPanel';

vi.mock('@tauri-apps/api/core', () => ({ convertFileSrc: (path: string) => path }));
vi.mock('@/shared/ui/liquid', () => ({
  LiquidSurface: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));
vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

function suggestion(overrides: Partial<DestinationSuggestion>): DestinationSuggestion {
  return {
    kind: 'create_canonical',
    objectId: null,
    canonicalEntryKey: 'backend-first',
    folderName: 'Backend First',
    targetPath: 'C:/Mods/Backend First',
    confidencePercentage: 60,
    confidenceTier: 'medium',
    matchMethod: 'canonical_identity',
    warning: null,
    ...overrides,
  };
}

function item(overrides: Partial<ImportItem>): ImportItem {
  return {
    id: 'item-1',
    batchId: 'batch-1',
    sourceKind: 'folder',
    sourcePath: 'C:/Downloads/mod',
    stagingPath: null,
    plannedName: 'mod',
    status: 'awaiting_destination',
    matchCategory: 'Character',
    subCategory: null,
    classificationMetadata: {},
    sourceMetadata: {},
    categorySuggestions: [],
    canonicalSuggestions: [],
    destinationSuggestions: [],
    selectedEntryKey: null,
    selectedAliasName: null,
    destinationObjectId: null,
    destinationPath: null,
    confidencePercentage: 0,
    confidenceTier: 'no_match',
    identityMatchStatus: 'no_match',
    evidence: [],
    decision: 'pending',
    fingerprint: null,
    archiveSha256: null,
    payloadManifest: null,
    duplicateOfItemId: null,
    targetComparison: null,
    analysisRevision: 0,
    analysisAckRevision: null,
    reviewGate: { reasons: [] },
    diagnostics: [],
    contentKind: 'unknown',
    packageShape: 'single',
    result: null,
    error: null,
    ...overrides,
  };
}

function batch(batchItem: ImportItem): ImportBatch {
  return {
    id: 'batch-1',
    gameId: 'game-1',
    flow: 'auto_import',
    targetMode: 'auto',
    targetObjectId: null,
    targetSubpath: null,
    status: 'awaiting_review',
    sourceArchivePath: null,
    items: [batchItem],
    createdAt: '2026-10-10T00:00:00Z',
    updatedAt: '2026-10-10T00:00:00Z',
  };
}

function object(id: string, name: string): ObjectSummary {
  return {
    id,
    name,
    folder_path: `C:/Mods/${name}`,
    matched_entry_key: null,
    matched_alias_name: null,
    matched_confidence: null,
    matched_reason: null,
    matched_source: null,
    object_type: 'Character',
    randomizer_mode: null,
    sub_category: null,
    status: 1,
    metadata: '{}',
    tags: '[]',
    hash_db: null,
    custom_skins: null,
    is_pinned: false,
    is_auto_sync: false,
    thumbnail_path: null,
    created_at: null,
    mod_count: 0,
    enabled_count: 0,
    safe_mod_count: 0,
    unsafe_mod_count: 0,
    unclassified_mod_count: 0,
    is_object_disabled: false,
    has_naming_conflict: false,
    active_mod_paths: null,
  };
}

function renderPanel(batchItem: ImportItem, objects: ObjectSummary[] = []) {
  render(
    <ImportBatchWizardDestinationPanel
      batch={batch(batchItem)}
      item={batchItem}
      objects={objects}
      busy={false}
      onChooseDestination={vi.fn().mockResolvedValue(undefined)}
      onChooseManualTarget={vi.fn().mockResolvedValue(undefined)}
    />,
  );
}

describe('ImportBatchWizardDestinationPanel', () => {
  it('keeps the backend suggestion order when showing the proposed destination', () => {
    const existing = suggestion({
      kind: 'existing_object',
      objectId: 'existing-object',
      canonicalEntryKey: 'existing',
      folderName: 'Existing High Confidence',
      targetPath: 'C:/Mods/Existing High Confidence',
      confidencePercentage: 99,
      confidenceTier: 'high',
    });
    const batchItem = item({
      destinationSuggestions: [suggestion({}), existing],
    });

    renderPanel(batchItem, [object('existing-object', 'Existing High Confidence')]);

    expect(screen.getByRole('button')).toHaveTextContent('Backend First');
  });

  it('shows the persisted create-canonical destination for a ready item', () => {
    const selected = suggestion({
      canonicalEntryKey: 'selected-entry',
      folderName: 'Selected Canonical',
      targetPath: 'C:/Mods/Selected Canonical',
      confidencePercentage: 80,
      confidenceTier: 'high',
    });
    const proposedExisting = suggestion({
      kind: 'existing_object',
      objectId: 'existing-object',
      canonicalEntryKey: 'existing',
      folderName: 'Existing Proposal',
      targetPath: 'C:/Mods/Existing Proposal',
      confidencePercentage: 95,
      confidenceTier: 'high',
    });
    const batchItem = item({
      status: 'ready',
      decision: 'create_canonical',
      selectedEntryKey: 'selected-entry',
      destinationPath: 'C:/Mods/Selected Canonical',
      destinationSuggestions: [proposedExisting, selected],
    });

    renderPanel(batchItem, [object('existing-object', 'Existing Proposal')]);

    expect(screen.getByRole('button')).toHaveTextContent('Selected Canonical');
    expect(screen.getByRole('button')).not.toHaveTextContent('Existing Proposal');
  });
});
