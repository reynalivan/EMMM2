import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { ImportBatch, ImportItem } from '../../shared/api/tauri/bindings.gen';
import { ImportBatchWizard } from './ImportBatchWizard';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, values?: { count?: number }) =>
      values?.count === undefined ? key : `${key}:${values.count}`,
  }),
}));

function item(overrides: Partial<ImportItem> = {}): ImportItem {
  return {
    id: 'item-1',
    batchId: 'batch-1',
    sourceKind: 'folder',
    sourcePath: 'C:/Downloads/DISABLED unknown-mod',
    stagingPath: null,
    plannedName: 'DISABLED unknown-mod',
    status: 'awaiting_category',
    matchCategory: null,
    subCategory: null,
    classificationMetadata: {},
    categorySuggestions: [],
    canonicalSuggestions: [],
    destinationSuggestions: [],
    selectedEntryKey: null,
    selectedAliasName: null,
    destinationObjectId: null,
    destinationPath: null,
    confidencePercentage: 0,
    confidenceTier: 'no_match',
    evidence: [],
    decision: 'pending',
    fingerprint: null,
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
    createdAt: '2026-08-28T00:00:00Z',
    updatedAt: '2026-08-28T00:00:00Z',
  };
}

function handlers() {
  return {
    onClassify: vi.fn().mockResolvedValue(undefined),
    onChooseDestination: vi.fn().mockResolvedValue(undefined),
    onChooseManualTarget: vi.fn().mockResolvedValue(undefined),
    onSkip: vi.fn().mockResolvedValue(undefined),
    onRename: vi.fn().mockResolvedValue(undefined),
    onRetry: vi.fn().mockResolvedValue(undefined),
    onOpenInExplorer: vi.fn().mockResolvedValue(undefined),
    onCommit: vi.fn().mockResolvedValue(undefined),
    onCancel: vi.fn().mockResolvedValue(undefined),
    onClose: vi.fn(),
  };
}

describe('ImportBatchWizard', () => {
  it('requires a category decision and accepts Other as metadata, not a destination', async () => {
    const batchItem = item();
    const callbacks = handlers();
    render(
      <ImportBatchWizard
        batch={batch(batchItem)}
        schema={null}
        objects={[]}
        busyItemId={null}
        report={null}
        {...callbacks}
      />,
    );

    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'Other' } });
    fireEvent.click(screen.getByRole('button', { name: 'actions.confirm' }));

    await waitFor(() =>
      expect(callbacks.onClassify).toHaveBeenCalledWith(batchItem, 'Other', null, {}),
    );
    expect(screen.queryByText(/Mods[/\\]Other/)).not.toBeInTheDocument();
    expect(callbacks.onCommit).not.toHaveBeenCalled();
  });

  it('bulk confirms only high-confidence destination suggestions without auto-commit', async () => {
    const suggestion = {
      kind: 'existing_object' as const,
      objectId: 'object-ayaka',
      canonicalEntryKey: 'ayaka',
      folderName: 'Ayaka',
      targetPath: 'C:/Mods/Ayaka/DISABLED ayaka-12319mods',
      confidencePercentage: 91,
      confidenceTier: 'high' as const,
      warning: null,
    };
    const batchItem = item({
      sourcePath: 'C:/Downloads/DISABLED ayaka-12319mods',
      plannedName: 'DISABLED ayaka-12319mods',
      status: 'awaiting_destination',
      matchCategory: 'Character',
      confidencePercentage: 91,
      confidenceTier: 'high',
      destinationSuggestions: [suggestion],
    });
    const callbacks = handlers();
    render(
      <ImportBatchWizard
        batch={batch(batchItem)}
        schema={null}
        objects={[]}
        busyItemId={null}
        report={null}
        {...callbacks}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'actions.confirm_high' }));

    await waitFor(() =>
      expect(callbacks.onChooseDestination).toHaveBeenCalledWith(batchItem, suggestion, 'confirm'),
    );
    expect(callbacks.onCommit).not.toHaveBeenCalled();
  });

  it('lets a skipped item resume by choosing a destination again', async () => {
    const suggestion = {
      kind: 'existing_object' as const,
      objectId: 'object-raiden',
      canonicalEntryKey: 'raiden-shogun',
      folderName: 'Raiden Shogun',
      targetPath: 'C:/Mods/Raiden Shogun/DISABLED shogun32114',
      confidencePercentage: 72,
      confidenceTier: 'medium' as const,
      warning: null,
    };
    const batchItem = item({
      status: 'skipped',
      matchCategory: 'Character',
      decision: 'skip',
      destinationSuggestions: [suggestion],
    });
    const callbacks = handlers();
    render(
      <ImportBatchWizard
        batch={batch(batchItem)}
        schema={null}
        objects={[]}
        busyItemId={null}
        report={null}
        {...callbacks}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Raiden Shogun' }));

    await waitFor(() =>
      expect(callbacks.onChooseDestination).toHaveBeenCalledWith(batchItem, suggestion, 'confirm'),
    );
  });

  it('keeps partial and metadata-pending items resumable from the wizard', async () => {
    const callbacks = handlers();
    render(
      <ImportBatchWizard
        batch={batch(
          item({
            status: 'metadata_pending',
            matchCategory: 'Character',
            destinationPath: 'C:/Mods/Ayaka/DISABLED skin',
            error: 'metadata write interrupted',
          }),
        )}
        schema={null}
        objects={[]}
        busyItemId={null}
        report={null}
        {...callbacks}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'actions.commit:1' }));
    await waitFor(() => expect(callbacks.onCommit).toHaveBeenCalledTimes(1));
    expect(screen.queryByRole('button', { name: 'actions.retry' })).not.toBeInTheDocument();
  });
});
