import { fireEvent, render, screen } from '@testing-library/react';
import type { ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';
import type { CanonicalClassificationCatalogEntry } from '../../../shared/api/tauri/bindings.gen';
import { CanonicalObjectCombobox } from './CanonicalObjectCombobox';

vi.mock('@/shared/ui/liquid', () => ({
  LiquidSurface: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

const entries: CanonicalClassificationCatalogEntry[] = [
  {
    entryKey: 'ayaka',
    name: 'Ayaka',
    category: 'Character',
    metadata: {},
    thumbnailPath: null,
    aliases: ['Kamisato Ayaka'],
  },
  {
    entryKey: 'raiden-shogun',
    name: 'Raiden Shogun',
    category: 'Character',
    metadata: {},
    thumbnailPath: null,
    aliases: ['Raiden'],
  },
];

describe('CanonicalObjectCombobox', () => {
  it('finds aliases and selects the active option with the keyboard', () => {
    const onSelect = vi.fn();
    render(
      <CanonicalObjectCombobox
        emptyLabel="No canonical matches"
        entries={entries}
        selectedEntryKey={null}
        suggestions={[]}
        onSelect={onSelect}
      />,
    );

    const input = screen.getByRole('combobox', { name: 'Canonical object' });
    fireEvent.focus(input);
    fireEvent.change(input, { target: { value: 'Raiden' } });
    expect(screen.getByRole('option', { name: /Raiden Shogun/ })).toBeInTheDocument();

    fireEvent.keyDown(input, { key: 'ArrowDown' });
    fireEvent.keyDown(input, { key: 'Enter' });

    expect(onSelect).toHaveBeenCalledWith('raiden-shogun');
  });
});
