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
        open={true}
        onClose={vi.fn()}
        objects={dummyObjects}
        currentObjectId="idle"
        currentStatus={true}
        onSubmit={onSubmit}
      />,
    );

    expect(screen.getByText('Move to Object')).toBeInTheDocument();
    expect(screen.getByText('Zeta')).toBeInTheDocument();

    // Search for Alpha
    fireEvent.change(screen.getByPlaceholderText('Search objects...'), {
      target: { value: 'alp' },
    });

    expect(screen.queryByText('Zeta')).toBeNull();
    expect(screen.getByText('Alpha')).toBeInTheDocument();

    // Select and submit
    fireEvent.click(screen.getByText('Alpha'));
    fireEvent.click(screen.getByText('Move'));

    expect(onSubmit).toHaveBeenCalledWith('2', 'disabled');
  });

  it('disables currentObject from being selected', () => {
    render(
      <MoveToObjectDialog
        open={true}
        onClose={vi.fn()}
        objects={dummyObjects}
        currentObjectId="2" // Alpha is current
        currentStatus={true}
        onSubmit={vi.fn()}
      />,
    );

    const alphaButton = screen.getByText('Alpha').closest('button');
    expect(alphaButton).toBeDisabled();
    expect(screen.getByText('(Current)')).toBeInTheDocument();
  });
});
