import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { ImportBatch, ImportItem } from '../../shared/api/tauri/bindings.gen';
import { ImportBatchWizard } from './ImportBatchWizard';

vi.mock('@tauri-apps/api/core', () => ({ convertFileSrc: (path: string) => path }));
vi.mock('@tanstack/react-virtual', () => ({
  useVirtualizer: (options: { count: number; estimateSize: () => number }) => {
    const size = options.estimateSize();
    return {
      getVirtualItems: () =>
        Array.from({ length: options.count }, (_, index) => ({
          index,
          start: index * size,
          end: (index + 1) * size,
        })),
      getTotalSize: () => options.count * size,
      measureElement: vi.fn(),
    };
  },
}));
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, values?: Record<string, string | number>) => {
      if (key === 'confidence_value') return `${values?.value}% · ${values?.label}`;
      return values?.count === undefined ? key : `${key}:${values.count}`;
    },
  }),
}));

function item(overrides: Partial<ImportItem> = {}): ImportItem {
  return {
    id: 'item-1',
    batchId: 'batch-1',
    sourceKind: 'folder',
    sourcePath: 'C:/Users/Test/Downloads/unknown-mod',
    stagingPath: null,
    plannedName: 'unknown-mod',
    status: 'awaiting_destination',
    matchCategory: 'Other',
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
    ...overrides,
    identityMatchStatus: overrides.identityMatchStatus ?? 'no_match',
    archiveSha256: overrides.archiveSha256 ?? null,
    payloadManifest: overrides.payloadManifest ?? null,
    duplicateOfItemId: overrides.duplicateOfItemId ?? null,
    targetComparison: overrides.targetComparison ?? null,
    analysisRevision: overrides.analysisRevision ?? 0,
    analysisAckRevision: overrides.analysisAckRevision ?? null,
    reviewGate: overrides.reviewGate ?? { reasons: [] },
    diagnostics: overrides.diagnostics ?? [],
    contentKind: overrides.contentKind ?? 'unknown',
    packageShape: overrides.packageShape ?? 'single',
    evidence: overrides.evidence ?? [],
    decision: overrides.decision ?? 'pending',
    fingerprint: overrides.fingerprint ?? null,
    result: overrides.result ?? null,
    error: overrides.error ?? null,
  };
}

function batch(batchItem: ImportItem, additionalItems: ImportItem[] = []): ImportBatch {
  return {
    id: 'batch-1',
    gameId: 'game-1',
    flow: 'auto_import',
    targetMode: 'auto',
    targetObjectId: null,
    targetSubpath: null,
    status: 'awaiting_review',
    sourceArchivePath: null,
    items: [batchItem, ...additionalItems],
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
  it('renders one review table without classification controls', () => {
    render(
      <ImportBatchWizard
        batch={batch(item())}
        schema={null}
        objects={[]}
        busyItemId={null}
        report={null}
        {...handlers()}
      />,
    );

    expect(screen.getByRole('table')).toBeInTheDocument();
    expect(screen.getByText('…/Downloads/unknown-mod')).toBeInTheDocument();
    expect(screen.queryByText('sections.match')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: /filters.no_match/ })).toBeInTheDocument();
  });

  it('edits the source mod name inline and requests rematching', async () => {
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

    fireEvent.click(screen.getByRole('button', { name: 'source.edit_name' }));
    fireEvent.change(screen.getByDisplayValue('unknown-mod'), { target: { value: 'Renamed mod' } });
    fireEvent.click(screen.getByRole('button', { name: 'actions.save_name' }));

    await waitFor(() => expect(callbacks.onRename).toHaveBeenCalledWith(batchItem, 'Renamed mod'));
  });

  it('supports select-all, select-none, and bulk proceed', async () => {
    const suggestion = {
      kind: 'existing_object' as const,
      objectId: 'object-ayaka',
      canonicalEntryKey: 'ayaka',
      folderName: 'Ayaka',
      targetPath: 'C:/Mods/Ayaka/skin',
      confidencePercentage: 91,
      confidenceTier: 'high' as const,
      warning: null,
    };
    const first = item({
      destinationSuggestions: [suggestion],
      canonicalSuggestions: [
        {
          entryKey: 'ayaka',
          name: 'Ayaka',
          matchedAlias: null,
          confidencePercentage: 91,
          confidenceTier: 'high',
          matchStatus: 'auto_matched',
          evidence: [],
        },
      ],
    });
    const second = item({ id: 'item-2', plannedName: 'second', sourcePath: 'C:/Mods/second' });
    const callbacks = handlers();
    render(
      <ImportBatchWizard
        batch={batch(first, [second])}
        schema={null}
        objects={[]}
        busyItemId={null}
        report={null}
        {...callbacks}
      />,
    );

    fireEvent.keyDown(window, { key: 'a', ctrlKey: true });
    fireEvent.click(screen.getByRole('button', { name: 'actions.set_proceed' }));

    await waitFor(() =>
      expect(callbacks.onChooseDestination).toHaveBeenCalledWith(first, suggestion, 'confirm'),
    );
    expect(callbacks.onCommit).not.toHaveBeenCalled();

    fireEvent.keyDown(window, { key: 'a', ctrlKey: true });
    fireEvent.keyDown(window, { key: 'a', ctrlKey: true, shiftKey: true });
    expect(screen.queryByRole('button', { name: 'actions.set_skip' })).not.toBeInTheDocument();
  });

  it('keeps review-gated items out of bulk proceed', async () => {
    const suggestion = {
      kind: 'existing_object' as const,
      objectId: 'object-ayaka',
      canonicalEntryKey: 'ayaka',
      folderName: 'Ayaka',
      targetPath: 'C:/Mods/Ayaka/skin',
      confidencePercentage: 91,
      confidenceTier: 'high' as const,
      warning: null,
    };
    const gated = item({
      destinationSuggestions: [suggestion],
      canonicalSuggestions: [
        {
          entryKey: 'ayaka',
          name: 'Ayaka',
          matchedAlias: null,
          confidencePercentage: 91,
          confidenceTier: 'high',
          matchStatus: 'auto_matched',
          evidence: [],
        },
      ],
      reviewGate: {
        reasons: [{ code: 'package_bundle', diagnosticCode: null }],
      },
    });
    const callbacks = handlers();
    render(
      <ImportBatchWizard
        batch={batch(gated)}
        schema={null}
        objects={[]}
        busyItemId={null}
        report={null}
        {...callbacks}
      />,
    );

    fireEvent.keyDown(window, { key: 'a', ctrlKey: true });
    fireEvent.click(screen.getByRole('button', { name: 'actions.set_proceed' }));

    await waitFor(() => expect(callbacks.onChooseDestination).not.toHaveBeenCalled());
  });

  it('groups duplicate target warnings under one review detail', () => {
    render(
      <ImportBatchWizard
        batch={batch(
          item({
            targetComparison: {
              outcome: 'incomplete',
              targetPath: 'C:/Mods/Ayaka/skin',
              sameFiles: 0,
              changedFiles: 0,
              missingFiles: 0,
              additionalFiles: 0,
              suggestedSeparateName: null,
              reason: 'target inspection incomplete',
            },
            reviewGate: {
              reasons: [{ code: 'target_comparison_incomplete', diagnosticCode: null }],
            },
          }),
        )}
        schema={null}
        objects={[]}
        busyItemId={null}
        report={null}
        {...handlers()}
      />,
    );

    expect(screen.getByText('review.required')).toBeInTheDocument();
    expect(screen.getByText('review.details:1')).toBeInTheDocument();
    expect(screen.getByText('target_comparison.incomplete')).toBeInTheDocument();
    expect(
      screen.queryByText('review_reasons.target_comparison_incomplete'),
    ).not.toBeInTheDocument();
  });

  it('labels an unselected canonical destination as a canonical match', () => {
    render(
      <ImportBatchWizard
        batch={batch(
          item({
            confidencePercentage: 92,
            confidenceTier: 'high',
            destinationSuggestions: [
              {
                kind: 'create_canonical',
                objectId: null,
                canonicalEntryKey: 'robin',
                folderName: 'Robin',
                targetPath: 'C:/Mods/Robin/RobinSummertoEdits',
                confidencePercentage: 92,
                confidenceTier: 'high',
                matchMethod: 'no_name_match',
                warning: null,
              },
            ],
          }),
        )}
        schema={null}
        objects={[]}
        busyItemId={null}
        report={null}
        {...handlers()}
      />,
    );

    expect(screen.getAllByText('match_methods.canonical_identity')).toHaveLength(2);
    expect(screen.queryByText('match_methods.no_name_match')).not.toBeInTheDocument();
  });

  it('shows the selected manual destination confidence instead of the source confidence', () => {
    render(
      <ImportBatchWizard
        batch={batch(
          item({
            decision: 'confirm',
            destinationObjectId: 'object-herta',
            destinationPath: 'C:/Mods/Herta/RobinSummertoEdits',
            confidencePercentage: 92,
            confidenceTier: 'high',
            destinationSuggestions: [
              {
                kind: 'create_canonical',
                objectId: null,
                canonicalEntryKey: 'robin',
                folderName: 'Robin',
                targetPath: 'C:/Mods/Robin/RobinSummertoEdits',
                confidencePercentage: 92,
                confidenceTier: 'high',
                matchMethod: 'no_name_match',
                warning: null,
              },
            ],
          }),
        )}
        schema={null}
        objects={[
          {
            id: 'object-herta',
            name: 'herta',
            folder_path: 'C:/Mods/Herta',
            matched_entry_key: null,
            matched_alias_name: null,
            matched_confidence: null,
            matched_reason: null,
            matched_source: null,
            object_type: 'Other',
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
          },
        ]}
        busyItemId={null}
        report={null}
        {...handlers()}
      />,
    );

    expect(screen.getByText('0% · confidence.no_match')).toBeInTheDocument();
    expect(screen.getByText('match_tooltip.manual')).toBeInTheDocument();
    expect(screen.queryByText('92% · confidence.high')).not.toBeInTheDocument();
  });

  it('selects a ranked destination from the searchable dropdown', async () => {
    const suggestion = {
      kind: 'existing_object' as const,
      objectId: 'object-raiden',
      canonicalEntryKey: 'raiden-shogun',
      folderName: 'Raiden Shogun',
      targetPath: 'C:/Mods/Raiden Shogun/skin',
      confidencePercentage: 72,
      confidenceTier: 'medium' as const,
      warning: null,
    };
    const batchItem = item({
      status: 'skipped',
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

    fireEvent.click(screen.getByRole('button', { name: /Raiden Shogun/ }));
    fireEvent.click(screen.getAllByRole('button', { name: /Raiden Shogun/ })[1]);

    await waitFor(() =>
      expect(callbacks.onChooseDestination).toHaveBeenCalledWith(batchItem, suggestion, 'confirm'),
    );
  });

  it('keeps recovery items commit-ready', async () => {
    const callbacks = handlers();
    render(
      <ImportBatchWizard
        batch={batch(
          item({
            status: 'metadata_pending',
            decision: 'confirm',
            destinationPath: 'C:/Mods/Ayaka/skin',
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

    fireEvent.click(screen.getAllByRole('button', { name: 'actions.resume' })[1]);
    await waitFor(() => expect(callbacks.onCommit).toHaveBeenCalledTimes(1));
  });

  it('localizes archive errors without exposing OS diagnostics', () => {
    render(
      <ImportBatchWizard
        batch={batch(
          item({
            status: 'failed',
            error:
              "Validation error: Unsupported archive: Extraction error: OS Error 42 (FormatMessageW returned error 317) 'Declared dictionary size is not supported'",
          }),
        )}
        schema={null}
        objects={[]}
        busyItemId={null}
        report={null}
        {...handlers()}
      />,
    );

    expect(screen.getByText('errors.archive.dictionary_too_large')).toBeInTheDocument();
    expect(screen.queryByText(/FormatMessageW/)).not.toBeInTheDocument();
  });
});
