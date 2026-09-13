import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import CatalogTab from './CatalogTab';

const mockSetCatalogAutoInstall = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({
    state: 'not_installed',
    pack_id: null,
    version: null,
    message: null,
    entries: 0,
    missing_assets: 0,
  }),
}));

vi.mock('@/entities/settings', () => ({
  useSettings: () => ({
    settings: { catalog_updates: { auto_install: false } },
    setCatalogAutoInstall: {
      mutate: mockSetCatalogAutoInstall,
      isPending: false,
    },
  }),
}));

describe('CatalogTab', () => {
  beforeEach(() => vi.clearAllMocks());

  it('keeps automatic catalog installation opt-in', () => {
    render(<CatalogTab />);

    const toggle = screen.getByRole('checkbox', { name: /Auto-install/i });
    expect(toggle).not.toBeChecked();
    fireEvent.click(toggle);
    expect(mockSetCatalogAutoInstall).toHaveBeenCalledWith(true);
  });
});
