import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { ContextMenu, ContextMenuItem } from './ContextMenu';
import { describe, it, expect, vi } from 'vitest';

describe('ContextMenu (Radix)', () => {
  it('renders children and shows menu on right click', async () => {
    const handleAction = vi.fn();

    render(
      <ContextMenu content={<ContextMenuItem onClick={handleAction}>Action Item</ContextMenuItem>}>
        <div data-testid="trigger">Right Click Me</div>
      </ContextMenu>,
    );

    // Initial state: menu hidden
    expect(screen.queryByText('Action Item')).toBeNull();

    // Trigger context menu
    fireEvent.contextMenu(screen.getByTestId('trigger'));

    // Menu should appear
    expect(await screen.findByText('Action Item')).toBeDefined();
  });

  it('fires onClick when item is selected', async () => {
    const handleAction = vi.fn();

    render(
      <ContextMenu content={<ContextMenuItem onClick={handleAction}>Click Me</ContextMenuItem>}>
        <div data-testid="trigger">Trigger</div>
      </ContextMenu>,
    );

    fireEvent.contextMenu(screen.getByTestId('trigger'));
    expect(await screen.findByText('Click Me')).toBeDefined();

    fireEvent.click(screen.getByText('Click Me'));
    expect(handleAction).toHaveBeenCalled();
  });

  it('closes on Escape key', async () => {
    render(
      <ContextMenu content={<ContextMenuItem>Item</ContextMenuItem>}>
        <div data-testid="trigger">Trigger</div>
      </ContextMenu>,
    );

    fireEvent.contextMenu(screen.getByTestId('trigger'));
    expect(await screen.findByText('Item')).toBeDefined();

    fireEvent.keyDown(document, { key: 'Escape' });

    await waitFor(() => {
      expect(screen.queryByText('Item')).toBeNull();
    });
  });
});
