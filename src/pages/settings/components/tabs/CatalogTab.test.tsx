import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  CatalogImportPreview,
  CatalogPackRefreshResult,
} from '@/shared/api/tauri/bindings.gen';
import CatalogTab from './CatalogTab';

vi.unmock('@tanstack/react-query');
const mocks = vi.hoisted(() => ({
  getCatalogPackStatus: vi.fn().mockResolvedValue({
    state: 'not_installed',
    pack_id: null,
    version: null,
    message: null,
    entries: 0,
  }),
  checkCatalogUpdate: vi.fn(),
  previewCatalogLocalImport: vi.fn(),
  installCatalogImport: vi.fn(),
  discardCatalogImport: vi.fn().mockResolvedValue(null),
}));
vi.mock('@/shared/api/tauri/bindings', () => ({ commands: mocks }));
vi.mock('@/entities/game', () => ({ useActiveGame: () => ({ activeGame: null }) }));

const preview: CatalogImportPreview = {
  stagingToken: 'review-token',
  sourceKind: 'local',
  sourceLabel: 'fixture.zip',
  sourceUrl: null,
  releaseTag: null,
  replacesActivePack: false,
  review: {
    packId: 'fixture',
    version: '1.0.0',
    publisher: 'Test',
    source: 'fixture',
    supportedGames: ['gimi'],
    entries: 1,
    hasKeyviewerTargets: false,
  },
};

function renderCatalog() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return {
    client,
    ...render(
      <QueryClientProvider client={client}>
        <CatalogTab />
      </QueryClientProvider>,
    ),
  };
}

async function openReview() {
  fireEvent.click(screen.getByRole('button', { name: /Choose ZIP/i }));
  await screen.findByRole('dialog');
}

describe('CatalogTab', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.previewCatalogLocalImport.mockResolvedValue(preview);
    mocks.installCatalogImport.mockResolvedValue({ state: 'ready', entries: 1 });
    mocks.discardCatalogImport.mockResolvedValue(null);
    mocks.checkCatalogUpdate.mockReset();
  });

  it('offers local ZIP review without an automatic-install preference', () => {
    renderCatalog();

    expect(screen.getByRole('button', { name: /Choose ZIP/i })).toBeEnabled();
    expect(screen.getByRole('button', { name: /Check now/i })).toBeDisabled();
    expect(screen.queryByRole('checkbox', { name: /Auto-install/i })).not.toBeInTheDocument();
  });

  it('enables update checks after a catalog pack is installed', async () => {
    mocks.getCatalogPackStatus.mockResolvedValueOnce({
      state: 'ready',
      pack_id: 'catalog-pack',
      version: '1.4.0',
      message: null,
      entries: 1,
    });
    renderCatalog();

    await waitFor(() => expect(screen.getByRole('button', { name: /Check now/i })).toBeEnabled());
  });

  it('shows why an installed pack cannot provide an update source', async () => {
    mocks.getCatalogPackStatus.mockResolvedValueOnce({
      state: 'ready',
      pack_id: 'catalog-pack',
      version: '1.4.0',
      message: null,
      entries: 1,
    });
    mocks.checkCatalogUpdate.mockRejectedValueOnce(
      new Error('Enter a public HTTPS GitHub repository or release URL'),
    );
    renderCatalog();
    const checkButton = screen.getByRole('button', { name: /Check now/i });
    await waitFor(() => expect(checkButton).toBeEnabled());
    fireEvent.click(checkButton);

    expect(await screen.findByText(/public HTTPS GitHub repository or release URL/i)).toBeVisible();
  });

  it('discards staged storage when review is cancelled', async () => {
    renderCatalog();
    await openReview();
    fireEvent.click(screen.getByRole('button', { name: /^Cancel$/i }));
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(mocks.discardCatalogImport).toHaveBeenCalledWith(preview.stagingToken);
  });

  it('blocks backdrop and Escape during installation, then refreshes catalog consumers', async () => {
    let finish!: (result: CatalogPackRefreshResult) => void;
    mocks.installCatalogImport.mockImplementationOnce(
      () =>
        new Promise<CatalogPackRefreshResult>((resolve) => {
          finish = resolve;
        }),
    );
    const { client } = renderCatalog();
    client.setQueryData(['master-db', 0], []);
    await openReview();
    fireEvent.click(screen.getByRole('button', { name: /^Install pack$/i }));
    await waitFor(() =>
      expect(mocks.installCatalogImport).toHaveBeenCalledWith(preview.stagingToken),
    );
    await waitFor(() => expect(screen.getByRole('button', { name: /^Close$/i })).toBeDisabled());
    expect(screen.getByRole('button', { name: /^Cancel$/i })).toBeDisabled();
    fireEvent(screen.getByRole('dialog'), new Event('cancel', { cancelable: true }));
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(mocks.discardCatalogImport).not.toHaveBeenCalled();
    finish({ state: 'ready', entries: 1 });
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    await waitFor(() => expect(client.getQueryState(['master-db', 0])?.isInvalidated).toBe(true));
  });

  it('retains the exact reviewed token after install failure for retry', async () => {
    mocks.installCatalogImport.mockRejectedValueOnce(new Error('Database is locked'));
    renderCatalog();
    await openReview();
    fireEvent.click(screen.getByRole('button', { name: /^Install pack$/i }));
    await screen.findByText(/Database is locked/);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /^Install pack$/i }));
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(mocks.installCatalogImport.mock.calls).toEqual([
      [preview.stagingToken],
      [preview.stagingToken],
    ]);
  });

  it('releases the reviewed staging token when settings unmounts', async () => {
    const view = renderCatalog();
    await openReview();
    view.unmount();
    await waitFor(() =>
      expect(mocks.discardCatalogImport).toHaveBeenCalledWith(preview.stagingToken),
    );
  });
});
