import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactElement } from 'react';
import { describe, it, expect, vi } from 'vitest';
import MoveToObjectDialog from './MoveToObjectDialog';
import type { ObjectSummary } from '../../types/object';

vi.unmock('@tanstack/react-query');

const listMoveTargetsForObject = vi.fn();

vi.mock('../../hooks/useActiveGame', () => ({
  useActiveGame: () => ({ activeGame: { id: 'g1' } }),
}));

vi.mock('../../lib/bindings', () => ({
  commands: {
    listMoveTargetsForObject: (params: { gameId: string; objectId: string }) =>
      listMoveTargetsForObject(params),
  },
}));

function renderDialog(ui: ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
}

describe('MoveToObjectDialog', () => {
  const dummyObjects = [
    { id: '1', name: 'Zeta', object_type: 'Character' },
    { id: '2', name: 'Alpha', object_type: 'Character' },
  ] as ObjectSummary[];

  it('filters objects and submits root move with keep status', async () => {
    listMoveTargetsForObject.mockReset();
    listMoveTargetsForObject.mockResolvedValue([
      {
        object_id: '2',
        object_name: 'Alpha',
        object_folder_path: 'Alpha',
        target_subpath: null,
        display_path: 'Alpha',
        depth: 0,
      },
    ]);
    const onSubmit = vi.fn();

    renderDialog(
      <MoveToObjectDialog
        isOpen={true}
        onClose={vi.fn()}
        objects={dummyObjects}
        currentObjectId="idle"
        targetModPaths={['mod/path']}
        onSubmit={onSubmit}
      />,
    );

    fireEvent.change(screen.getByPlaceholderText('Search objects...'), {
      target: { value: 'alp' },
    });
    expect(screen.queryByText('Zeta')).toBeNull();

    fireEvent.click(screen.getByText('Alpha'));
    await screen.findByText('Alpha');
    fireEvent.click(screen.getByText('common:actions.move'));

    await waitFor(() => expect(onSubmit).toHaveBeenCalledWith('2', 'keep', null));
  });

  it('allows moving within the current object and selecting an existing subfolder', async () => {
    listMoveTargetsForObject.mockReset();
    listMoveTargetsForObject.mockResolvedValue([
      {
        object_id: '2',
        object_name: 'Alpha',
        object_folder_path: 'Alpha',
        target_subpath: null,
        display_path: 'Alpha',
        depth: 0,
      },
      {
        object_id: '2',
        object_name: 'Alpha',
        object_folder_path: 'Alpha',
        target_subpath: 'Variants',
        display_path: 'Alpha/Variants',
        depth: 1,
      },
    ]);
    const onSubmit = vi.fn();

    renderDialog(
      <MoveToObjectDialog
        isOpen={true}
        onClose={vi.fn()}
        objects={dummyObjects}
        currentObjectId="2"
        targetModPaths={['mod/path']}
        onSubmit={onSubmit}
      />,
    );

    fireEvent.click(screen.getByText('Alpha'));
    await waitFor(() =>
      expect(listMoveTargetsForObject).toHaveBeenCalledWith({ gameId: 'g1', objectId: '2' }),
    );
    fireEvent.click(await screen.findByText('Alpha/Variants'));
    fireEvent.click(screen.getByText('Disabled'));
    fireEvent.click(screen.getByText('common:actions.move'));

    await waitFor(() => expect(onSubmit).toHaveBeenCalledWith('2', 'disabled', 'Variants'));
    expect(screen.getByText('(Current)')).toBeInTheDocument();
  });
});
