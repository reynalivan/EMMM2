import { fireEvent, render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import BulkProgressBar from './BulkProgressBar';
import { useBulkProgress } from '../hooks/useBulkProgress';
import { commands } from '../../../shared/api/tauri/bindings';

vi.mock('../hooks/useBulkProgress', () => ({
  useBulkProgress: vi.fn(),
}));

vi.mock('../../../shared/api/tauri/bindings', () => ({
  commands: { bulkCancel: vi.fn().mockResolvedValue({ status: 'ok', data: null }) },
}));

describe('BulkProgressBar', () => {
  it('does not render when inactive', () => {
    vi.mocked(useBulkProgress).mockReturnValue({
      operation_id: '',
      cancellable: false,
      active: false,
      label: '',
      current: 0,
      total: 0,
    });
    const { container } = render(<BulkProgressBar />);
    expect(container.firstChild).toBeNull();
  });

  it('renders progress correctly when active', () => {
    vi.mocked(useBulkProgress).mockReturnValue({
      operation_id: 'toggle-1',
      cancellable: true,
      active: true,
      label: 'Processing Files',
      current: 5,
      total: 10,
    });
    render(<BulkProgressBar />);

    expect(screen.getByText('Processing Files')).toBeInTheDocument();
    expect(screen.getByText('5 / 10')).toBeInTheDocument();
    expect(screen.getByRole('progressbar')).toHaveAttribute('value', '5');
    expect(screen.getByRole('progressbar')).toHaveAttribute('max', '10');
  });

  it('caps displayed current count to total', () => {
    vi.mocked(useBulkProgress).mockReturnValue({
      operation_id: 'toggle-1',
      cancellable: true,
      active: true,
      label: 'Processing',
      current: 15,
      total: 10,
    });
    render(<BulkProgressBar />);

    expect(screen.getByText('10 / 10')).toBeInTheDocument();
    expect(screen.getByRole('progressbar')).toHaveAttribute('value', '15'); // HTML progress can take higher values, logic limits display
  });

  it('cancels the running batch when the cancel button is clicked', () => {
    vi.mocked(useBulkProgress).mockReturnValue({
      operation_id: 'toggle-1',
      cancellable: true,
      active: true,
      label: 'Disabling 5000 mods...',
      current: 500,
      total: 5000,
    });
    render(<BulkProgressBar />);

    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));

    expect(commands.bulkCancel).toHaveBeenCalledTimes(1);
    expect(commands.bulkCancel).toHaveBeenCalledWith('toggle-1');
  });

  it('does not offer cancellation for an atomic workspace switch', () => {
    vi.mocked(useBulkProgress).mockReturnValue({
      operation_id: 'workspace-switch',
      cancellable: false,
      active: true,
      label: 'Enabling parent folders',
      current: 1,
      total: 2,
    });

    render(<BulkProgressBar />);

    expect(screen.queryByRole('button', { name: 'Cancel' })).not.toBeInTheDocument();
  });
});
