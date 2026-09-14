import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  CanonicalClassificationCatalogEntry,
  ObjectClassificationPreviewItem,
} from '../../../shared/api/tauri/bindings.gen';
import { openObjectClassificationWizard } from '@/features/import-batches/classificationLauncher';
import { ObjectClassificationWizardHost } from './ObjectClassificationWizardHost';

const mocks = vi.hoisted(() => ({
  applyObjectClassificationBatch: vi.fn(),
  getGames: vi.fn(),
  listCanonicalClassificationCatalog: vi.fn(),
  toastError: vi.fn(),
  toastSuccess: vi.fn(),
  publishQueryScopes: vi.fn(),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, values?: { count?: number; name?: string; mods?: number }) => {
      if (values?.count !== undefined) return `${key}:${values.count}`;
      if (values?.name !== undefined) return `${key}:${values.name}`;
      return key;
    },
  }),
  initReactI18next: { type: '3rdParty', init: vi.fn() },
}));
vi.mock('../../../shared/api/tauri/bindings', () => ({
  commands: {
    applyObjectClassificationBatch: mocks.applyObjectClassificationBatch,
    getGames: mocks.getGames,
    listCanonicalClassificationCatalog: mocks.listCanonicalClassificationCatalog,
  },
}));
vi.mock('@/shared/ui/toast', () => ({
  toast: { error: mocks.toastError, success: mocks.toastSuccess, warning: vi.fn() },
}));
vi.mock('@/shared/lib/queryRefresh', () => ({ publishQueryScopes: mocks.publishQueryScopes }));
vi.mock('@/shared/ui/components/ui/VirtualList', () => ({
  default: ({
    items,
    renderItem,
  }: {
    items: unknown[];
    renderItem: (item: unknown) => ReactNode;
  }) => <div>{items.map(renderItem)}</div>,
}));

const ayaka: CanonicalClassificationCatalogEntry = {
  entryKey: 'ayaka',
  name: 'Ayaka',
  category: 'Character',
  metadata: { element: 'Cryo', rarity: '5-Star' },
  thumbnailPath: null,
  aliases: ['Kamisato Ayaka'],
};

const raiden: CanonicalClassificationCatalogEntry = {
  entryKey: 'raiden-shogun',
  name: 'Raiden Shogun',
  category: 'Character',
  metadata: { element: 'Electro', rarity: '5-Star' },
  thumbnailPath: null,
  aliases: ['Raiden'],
};

function previewItem(
  objectId: string,
  objectName: string,
  suggestion: ObjectClassificationPreviewItem['canonicalSuggestions'][number] | null,
): ObjectClassificationPreviewItem {
  return {
    objectId,
    objectName,
    sourcePath: `C:/Mods/${objectName}`,
    currentCategory: 'Other',
    canonicalSuggestions: suggestion ? [suggestion] : [],
    fingerprint: {
      path: `C:/Mods/${objectName}`,
      modifiedUnixMs: '1',
      sizeBytes: '1',
      fileCount: 1,
    },
  };
}

function renderHost() {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={queryClient}>
      <ObjectClassificationWizardHost />
    </QueryClientProvider>,
  );
}

async function openWizard(items: ObjectClassificationPreviewItem[]) {
  act(() =>
    openObjectClassificationWizard({
      gameId: 'game-1',
      objectIds: items.map((item) => item.objectId),
      initialItems: items,
    }),
  );
  await screen.findByRole('dialog', { name: 'classification_title' });
}

describe('ObjectClassificationWizardHost', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.getGames.mockResolvedValue([]);
    mocks.listCanonicalClassificationCatalog.mockResolvedValue([ayaka, raiden]);
    mocks.applyObjectClassificationBatch.mockResolvedValue({
      applied: 1,
      childModsUpdated: 0,
      aliasesChanged: false,
      disabledObjects: 0,
      disableWarning: null,
    });
    mocks.publishQueryScopes.mockResolvedValue(undefined);
  });

  it('bulk-selects only AutoMatched high-confidence recommendations and applies selected rows', async () => {
    renderHost();
    await openWizard([
      previewItem('object-ayaka', 'Ayaka source', {
        entryKey: 'ayaka',
        name: 'Ayaka',
        matchedAlias: null,
        confidencePercentage: 91,
        confidenceTier: 'high',
        matchStatus: 'auto_matched',
        evidence: [],
      }),
      previewItem('object-raiden', 'Raiden source', {
        entryKey: 'raiden-shogun',
        name: 'Raiden Shogun',
        matchedAlias: 'Raiden',
        confidencePercentage: 93,
        confidenceTier: 'high',
        matchStatus: 'needs_review',
        evidence: [],
      }),
    ]);

    fireEvent.click(screen.getByRole('button', { name: 'classification_select_high_confidence' }));

    expect(screen.getByLabelText('classification_select_row:Ayaka source')).toBeChecked();
    expect(screen.getByLabelText('classification_select_row:Raiden source')).not.toBeChecked();

    fireEvent.click(screen.getByRole('button', { name: 'classification_apply_selected:1' }));

    await waitFor(() =>
      expect(mocks.applyObjectClassificationBatch).toHaveBeenCalledWith({
        gameId: 'game-1',
        disableAfterApply: false,
        items: [
          {
            objectId: 'object-ayaka',
            decision: { kind: 'canonical', entryKey: 'ayaka' },
            fingerprint: expect.any(Object),
          },
        ],
      }),
    );
  });

  it('uses manual classification only after the fallback is chosen', async () => {
    renderHost();
    await openWizard([previewItem('object-unknown', 'Unknown source', null)]);

    fireEvent.click(screen.getByRole('button', { name: 'classification_manual' }));
    fireEvent.click(screen.getByLabelText('classification_select_row:Unknown source'));
    fireEvent.click(screen.getByRole('button', { name: 'classification_apply_selected:1' }));

    await waitFor(() =>
      expect(mocks.applyObjectClassificationBatch).toHaveBeenCalledWith(
        expect.objectContaining({
          items: [
            expect.objectContaining({
              decision: { kind: 'manual', category: 'Other', subCategory: null, metadata: {} },
            }),
          ],
        }),
      ),
    );
  });
});
