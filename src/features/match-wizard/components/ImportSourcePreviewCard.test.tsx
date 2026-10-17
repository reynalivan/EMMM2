import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ImportSourcePreviewCard } from './ImportSourcePreviewCard';
import type { ImportItem, ImportSourcePreview } from '../../../shared/api/tauri/bindings.gen';

type LoadPreview = (item: ImportItem) => Promise<ImportSourcePreview>;

const item: ImportItem = {
  id: 'item-1',
  batchId: 'batch-1',
  sourceKind: 'folder',
  sourcePath: 'C:/Inbox/Source',
  stagingPath: null,
  plannedName: 'Source',
  status: 'ready',
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
};

const preview: ImportSourcePreview = {
  itemId: item.id,
  thumbnailPath: null,
  imageThumbnails: [],
  entries: [],
  folderCount: 0,
  fileCount: 0,
  totalSizeBytes: 0,
  truncated: false,
};

describe('ImportSourcePreviewCard', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => {
    cleanup();
    vi.clearAllTimers();
    vi.useRealTimers();
  });

  it.each(['hover', 'focus'] as const)(
    'loads a preview after sustained %s',
    async (interaction) => {
      const loadPreview = vi.fn<LoadPreview>().mockResolvedValue(preview);
      render(
        <ImportSourcePreviewCard item={item} loadPreview={loadPreview}>
          <button>Source</button>
        </ImportSourcePreviewCard>,
      );
      const button = screen.getByRole('button');
      if (interaction === 'hover') fireEvent.mouseEnter(button.parentElement!);
      else fireEvent.focus(button);
      await act(async () => {
        await vi.advanceTimersByTimeAsync(300);
      });
      expect(screen.getByRole('complementary')).toBeInTheDocument();
      expect(loadPreview).toHaveBeenCalledExactlyOnceWith(item);
      if (interaction === 'hover') fireEvent.mouseLeave(button.parentElement!);
      else fireEvent.blur(button);
      expect(screen.queryByRole('complementary')).not.toBeInTheDocument();
    },
  );

  it('cancels overlapping mouse and focus timers after both interactions end', async () => {
    const loadPreview = vi.fn<LoadPreview>().mockResolvedValue(preview);
    render(
      <ImportSourcePreviewCard item={item} loadPreview={loadPreview}>
        <button>Source</button>
      </ImportSourcePreviewCard>,
    );
    const button = screen.getByRole('button');
    fireEvent.mouseEnter(button.parentElement!);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(100);
    });
    fireEvent.focus(button);
    fireEvent.mouseLeave(button.parentElement!);
    fireEvent.blur(button);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(400);
    });
    expect(screen.queryByRole('complementary')).not.toBeInTheDocument();
    expect(loadPreview).not.toHaveBeenCalled();
  });

  it('cancels delayed preview loading on unmount', async () => {
    const loadPreview = vi.fn<LoadPreview>().mockResolvedValue(preview);
    const view = render(
      <ImportSourcePreviewCard item={item} loadPreview={loadPreview}>
        <button>Source</button>
      </ImportSourcePreviewCard>,
    );
    fireEvent.focus(screen.getByRole('button'));
    view.unmount();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(400);
    });
    expect(loadPreview).not.toHaveBeenCalled();
  });

  it.each([
    { count: 12, truncated: false, notice: false },
    { count: 13, truncated: false, notice: true },
    { count: 12, truncated: true, notice: true },
  ])(
    'discloses omissions for $count entries and truncated=$truncated',
    async ({ count, truncated, notice }) => {
      const loadPreview = vi.fn<LoadPreview>().mockResolvedValue({
        ...preview,
        entries: Array.from({ length: count }, (_, index) => ({
          relativePath: `source-${index}.ini`,
          kind: 'file',
          depth: 0,
        })),
        fileCount: count,
        truncated,
      } satisfies ImportSourcePreview);
      render(
        <ImportSourcePreviewCard item={item} loadPreview={loadPreview}>
          <button>Source</button>
        </ImportSourcePreviewCard>,
      );
      fireEvent.focus(screen.getByRole('button'));
      await act(async () => {
        await vi.advanceTimersByTimeAsync(300);
      });
      expect(screen.getByText('source-11.ini')).toBeInTheDocument();
      expect(screen.queryByText('source-12.ini')).not.toBeInTheDocument();
      const omissionNotice = screen.queryByText('source_preview.more');
      if (notice) {
        expect(omissionNotice).toBeInTheDocument();
        expect(omissionNotice?.closest('.overflow-hidden')).toBeNull();
        expect(omissionNotice?.closest('.max-h-40')).toBeNull();
      } else expect(omissionNotice).not.toBeInTheDocument();
    },
  );
});
