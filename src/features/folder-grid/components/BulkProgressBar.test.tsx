import { fireEvent, render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import BulkProgressBar from './BulkProgressBar';
import { useBulkProgress } from '../hooks/useBulkProgress';
import { commands } from '../../../lib/bindings';

vi.mock('../hooks/useBulkProgress', () => ({
  useBulkProgress: vi.fn(),
}));

vi.mock('../../../lib/bindings', () => ({
  commands: { bulkCancel: vi.fn().mockResolvedValue({ status: 'ok', data: null }) },
}));

describe('BulkProgressBar', () => {
  it('does not render when inactive', () => {
    vi.mocked(useBulkProgress).mockReturnValue({ active: false, label: '', current: 0, total: 0 });
    const { container } = render(<BulkProgressBar />);
    expect(container.firstChild).toBeNull();
  });

  it('renders progress correctly when active', () => {
    vi.mocked(useBulkProgress).mockReturnValue({
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
      active: true,
      label: 'Disabling 5000 mods...',
      current: 500,
      total: 5000,
    });
    render(<BulkProgressBar />);

    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));

    expect(commands.bulkCancel).toHaveBeenCalledTimes(1);
  });
});
