import React from 'react';
import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import ObjectListModals, { SYNC_CONFIRM_RESET } from './ObjectListModals';

vi.mock('../../components/ui/ConfirmDialog', () => ({
  default: ({ open }: { open: boolean }) =>
    open ? <div data-testid="confirm-dialog">Confirm Dialog</div> : null,
}));
vi.mock('./EditObjectModal', () => ({
  default: ({ open }: { open: boolean }) =>
    open ? <div data-testid="edit-modal">Edit Modal</div> : null,
}));
vi.mock('./SyncConfirmModal', () => ({
  default: ({ open }: { open: boolean }) =>
    open ? <div data-testid="sync-modal">Sync Modal</div> : null,
}));
vi.mock('./CreateObjectModal', () => ({
  default: ({ open }: { open: boolean }) =>
    open ? <div data-testid="create-modal">Create Modal</div> : null,
}));
vi.mock('./ScanReviewModal', () => ({
  default: ({ open }: { open: boolean }) =>
    open ? <div data-testid="scan-modal">Scan Modal</div> : null,
}));

describe('ObjectListModals', () => {
  it('renders nothing when not explicitly opened', () => {
    render(
      <ObjectListModals
        activeGame={null}
        deleteDialog={{ open: false, path: '', name: '', itemCount: 1 }}
        onConfirmDelete={vi.fn()}
        onCancelDelete={vi.fn()}
        editObject={null}
        onCloseEdit={vi.fn()}
        syncConfirm={SYNC_CONFIRM_RESET}
        onApplySyncMatch={vi.fn()}
        onEditManually={vi.fn()}
        onCloseSyncConfirm={vi.fn()}
        scanReview={{ open: false, items: [], masterDbEntries: [], isCommitting: false }}
        onCommitScan={vi.fn()}
        onCloseScanReview={vi.fn()}
        createModalOpen={false}
        onCloseCreate={vi.fn()}
      />,
    );
    expect(screen.queryByTestId('confirm-dialog')).toBeNull();
  });

  it('renders modals when opened', () => {
    render(
      <ObjectListModals
        activeGame={null}
        deleteDialog={{ open: true, path: '', name: '', itemCount: 1 }}
        onConfirmDelete={vi.fn()}
        onCancelDelete={vi.fn()}
        editObject={
          { id: '1', name: 'Z' } as unknown as React.ComponentProps<
            typeof ObjectListModals
          >['editObject']
        }
        onCloseEdit={vi.fn()}
        syncConfirm={{ ...SYNC_CONFIRM_RESET, open: true }}
        onApplySyncMatch={vi.fn()}
        onEditManually={vi.fn()}
        onCloseSyncConfirm={vi.fn()}
        scanReview={{ open: true, items: [], masterDbEntries: [], isCommitting: false }}
        onCommitScan={vi.fn()}
        onCloseScanReview={vi.fn()}
        createModalOpen={true}
        onCloseCreate={vi.fn()}
      />,
    );

    expect(screen.getByTestId('confirm-dialog')).toBeInTheDocument();
    expect(screen.getByTestId('edit-modal')).toBeInTheDocument();
    expect(screen.getByTestId('sync-modal')).toBeInTheDocument();
    expect(screen.getByTestId('scan-modal')).toBeInTheDocument();
    expect(screen.getByTestId('create-modal')).toBeInTheDocument();
  });
});
