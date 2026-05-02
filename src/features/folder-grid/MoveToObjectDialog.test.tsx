import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import MoveToObjectDialog from './MoveToObjectDialog';
import type { ObjectSummary } from '../../types/object';

describe('MoveToObjectDialog', () => {
  const dummyObjects = [
    { id: '1', name: 'Zeta', object_type: 'Character' },
    { id: '2', name: 'Alpha', object_type: 'Character' },
  ] as ObjectSummary[];

  it('renders correctly and filters objects based on search', () => {
    const onSubmit = vi.fn();
    render(
      <MoveToObjectDialog
        isOpen={true}
        onClose={vi.fn()}
        objects={dummyObjects}
        currentObjectId="idle"
        targetModPaths={['mod/path']}
        onSubmit={onSubmit}
      />,
    );

    expect(screen.getByText('folder_grid:move.title')).toBeInTheDocument();
    expect(screen.getByText('Zeta')).toBeInTheDocument();

    // Search for Alpha
    fireEvent.change(screen.getByPlaceholderText('folder_grid:move.placeholder'), {
      target: { value: 'alp' },
    });

    expect(screen.queryByText('Zeta')).toBeNull();
    expect(screen.getByText('Alpha')).toBeInTheDocument();

    // Select and submit
    fireEvent.click(screen.getByText('Alpha'));
    fireEvent.click(screen.getByText('common:actions.move'));

    expect(onSubmit).toHaveBeenCalledWith('2', 'disabled');
  });

  it('disables currentObject from being selected', () => {
    render(
      <MoveToObjectDialog
        isOpen={true}
        onClose={vi.fn()}
        objects={dummyObjects}
        currentObjectId="2" // Alpha is current
        targetModPaths={['mod/path']}
        onSubmit={vi.fn()}
      />,
    );

    const alphaButton = screen.getByText('Alpha').closest('button');
    expect(alphaButton).toBeDisabled();
    expect(screen.getByText('folder_grid:move.current_marker')).toBeInTheDocument();
  });
});
