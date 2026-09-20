import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '../../../tests/testing/test-utils';
import type { ActiveKeyBinding } from '@/entities/settings';
import type { GameConfig } from '@/entities/game';
import { DashboardActivity } from './DashboardActivity';

vi.mock('@/entities/mod', () => ({
  ModThumbnail: () => null,
}));

const activeGame = { id: 'game-1' } as GameConfig;

const keybindings: ActiveKeyBinding[] = [
  {
    mod_name: 'Outfit Pack',
    folder_path: 'Characters/Raiden/Outfit Pack',
    object_type: 'Character',
    matched_alias_name: 'Raiden Shogun',
    section_name: 'Outfit',
    key: 'F1',
    back: null,
    control_kind: 'key_binding',
    value_summary: null,
  },
  {
    mod_name: 'Blade Pack',
    folder_path: 'Weapons/Mistsplitter/Blade Pack',
    object_type: 'Weapon',
    matched_alias_name: 'Mistsplitter',
    section_name: 'Blade',
    key: 'F2',
    back: null,
    control_kind: 'key_binding',
    value_summary: null,
  },
];

describe('DashboardActivity active key mappings', () => {
  it('searches by database alias and mod folder path', () => {
    render(
      <DashboardActivity
        activeGame={activeGame}
        keybindings={keybindings}
        keybindingsLoading={false}
        recentMods={[]}
      />,
    );

    const search = screen.getByRole('searchbox', { name: 'Search active key mappings' });

    fireEvent.change(search, { target: { value: 'Shogun' } });
    expect(screen.getAllByText('Outfit Pack')).toHaveLength(2);
    expect(screen.queryByText('Blade Pack')).not.toBeInTheDocument();

    fireEvent.change(search, { target: { value: 'Mistsplitter' } });
    expect(screen.getAllByText('Blade Pack')).toHaveLength(2);
    expect(screen.queryByText('Outfit Pack')).not.toBeInTheDocument();
  });

  it('filters by classification and clears filters', () => {
    render(
      <DashboardActivity
        activeGame={activeGame}
        keybindings={keybindings}
        keybindingsLoading={false}
        recentMods={[]}
      />,
    );

    const filter = screen.getByRole('combobox', { name: 'Filter by classification' });
    fireEvent.change(filter, { target: { value: 'Character' } });

    expect(screen.getAllByText('Outfit Pack')).toHaveLength(2);
    expect(screen.queryByText('Blade Pack')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Clear filters' }));
    expect(screen.getAllByText('Outfit Pack')).toHaveLength(2);
    expect(screen.getAllByText('Blade Pack')).toHaveLength(2);
  });
});
