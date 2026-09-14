import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import CatalogTab from './CatalogTab';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({
    state: 'not_installed',
    pack_id: null,
    version: null,
    message: null,
    entries: 0,
  }),
}));

describe('CatalogTab', () => {
  beforeEach(() => vi.clearAllMocks());

  it('offers local ZIP review without an automatic-install preference', () => {
    render(<CatalogTab />);

    expect(screen.getByRole('button', { name: /Choose ZIP/i })).toBeEnabled();
    expect(screen.queryByRole('checkbox', { name: /Auto-install/i })).not.toBeInTheDocument();
  });
});
