import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ExtractionEvent, ImportBatch } from '../../shared/api/tauri/bindings.gen';
import { ImportBatchWizardHost } from './ImportBatchWizardHost';

const mocks = vi.hoisted(() => {
  class MockChannel<T> {
    onmessage: ((message: T) => void) | null = null;
  }

  return {
    MockChannel,
    analyzeImportBatch: vi.fn(),
    analyzeImportBatchWithOptions: vi.fn(),
    createImportBatch: vi.fn(),
    getGames: vi.fn(),
    getImportBatch: vi.fn(),
    getObjectsCmd: vi.fn(),
    listImportBatches: vi.fn(),
    toastError: vi.fn(),
  };
});

vi.mock('@tauri-apps/api/core', () => ({ Channel: mocks.MockChannel }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(vi.fn()),
}));
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, options?: { current?: number; total?: number }) =>
      options?.current === undefined || options.total === undefined
        ? key
        : `${key}:${options.current}:${options.total}`,
  }),
}));
vi.mock('../../shared/api/tauri/bindings', () => ({
  commands: {
    analyzeImportBatch: mocks.analyzeImportBatch,
    analyzeImportBatchWithOptions: mocks.analyzeImportBatchWithOptions,
    createImportBatch: mocks.createImportBatch,
    getGames: mocks.getGames,
    getImportBatch: mocks.getImportBatch,
    getObjectsCmd: mocks.getObjectsCmd,
    listImportBatches: mocks.listImportBatches,
  },
}));
vi.mock('@/app/store', () => ({
  useAppStore: (selector: (state: { activeGameId: string | null }) => unknown) =>
    selector({ activeGameId: 'game-1' }),
}));
vi.mock('@/shared/ui/toast', () => ({
  toast: { error: mocks.toastError },
}));
vi.mock('../match-wizard/ImportBatchWizard', () => ({
  ImportBatchWizard: ({ batch }: { batch: ImportBatch }) => (
    <div data-testid="import-batch-wizard">{batch.status}</div>
  ),
}));

function batch(status: ImportBatch['status']): ImportBatch {
  return {
    id: 'batch-1',
    gameId: 'game-1',
    flow: 'auto_import',
    targetMode: 'auto',
    targetObjectId: null,
    targetSubpath: null,
    status,
    sourceArchivePath: 'C:/Downloads/mod.zip',
    createdAt: '2026-08-30T00:00:00Z',
    updatedAt: '2026-08-30T00:00:00Z',
    items: [
      {
        id: 'item-1',
        batchId: 'batch-1',
        sourceKind: 'archive_root',
        sourcePath: 'C:/Downloads/mod.zip',
        stagingPath: null,
        plannedName: 'mod',
        status: 'discovered',
        matchCategory: null,
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
      },
    ],
  };
}

async function resumeImport(): Promise<void> {
  fireEvent.click(await screen.findByRole('button', { name: 'resume_import' }));
}

describe('ImportBatchWizardHost archive analysis', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.createImportBatch.mockResolvedValue(batch('draft'));
    mocks.getImportBatch.mockResolvedValue(batch('draft'));
    mocks.getObjectsCmd.mockResolvedValue({ objects: [] });
    mocks.getGames.mockResolvedValue([]);
    mocks.listImportBatches.mockResolvedValue([batch('draft')]);
  });

  it('uses the options command with nested extraction enabled and surfaces channel progress', async () => {
    let resolveAnalysis: (value: ImportBatch) => void = () => undefined;
    mocks.analyzeImportBatchWithOptions.mockImplementation(
      () => new Promise<ImportBatch>((resolve) => (resolveAnalysis = resolve)),
    );
    render(<ImportBatchWizardHost />);

    await resumeImport();

    await waitFor(() =>
      expect(mocks.analyzeImportBatchWithOptions).toHaveBeenCalledWith(
        { batchId: 'batch-1', password: null, unpackNested: true },
        expect.any(mocks.MockChannel),
      ),
    );
    expect(mocks.analyzeImportBatch).not.toHaveBeenCalled();
    expect(document.querySelector('.fixed.inset-0')).toHaveClass(
      'z-[var(--workspace-layer-overlay)]',
    );

    const channel = mocks.analyzeImportBatchWithOptions.mock.calls[0][1] as InstanceType<
      typeof mocks.MockChannel<ExtractionEvent>
    >;
    act(() => {
      channel.onmessage?.({
        event: 'fileProgress',
        data: { fileName: 'nested/mod.ini', fileIndex: 2, totalFiles: 4 },
      });
    });

    expect(screen.getByRole('status')).toHaveTextContent('extraction.progress:2:4');
    expect(screen.getByRole('status')).toHaveTextContent('nested/mod.ini');

    await act(async () => resolveAnalysis(batch('awaiting_review')));
    expect(await screen.findByTestId('import-batch-wizard')).toHaveTextContent('awaiting_review');
  });

  it('prompts for an archive password and retries without retaining it in the input', async () => {
    let resolveRetry: (value: ImportBatch) => void = () => undefined;
    mocks.analyzeImportBatchWithOptions
      .mockRejectedValueOnce({ type: 'ArchivePasswordRequired' })
      .mockImplementationOnce(
        () => new Promise<ImportBatch>((resolve) => (resolveRetry = resolve)),
      );
    render(<ImportBatchWizardHost />);

    await resumeImport();

    const dialog = await screen.findByRole('dialog', { name: 'archive_password.title' });
    const passwordInput = screen.getByLabelText('archive_password.label');
    fireEvent.change(passwordInput, { target: { value: 'correct horse battery staple' } });
    fireEvent.submit(dialog.querySelector('form')!);

    await waitFor(() => expect(mocks.analyzeImportBatchWithOptions).toHaveBeenCalledTimes(2));
    expect(mocks.analyzeImportBatchWithOptions.mock.calls[1][0]).toEqual({
      batchId: 'batch-1',
      password: 'correct horse battery staple',
      unpackNested: true,
    });
    expect(passwordInput).toHaveValue('');

    await act(async () => resolveRetry(batch('awaiting_review')));
    await waitFor(() =>
      expect(screen.queryByRole('dialog', { name: 'archive_password.title' })).toBeNull(),
    );
  });
});
