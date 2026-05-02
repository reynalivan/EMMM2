/**
 * Tests for RandomizerModal component.
 * Covers: TC-35-001 (Roll Luck / Proposal Generation),
 *         TC-35-002 (Reroll), TC-35-003 (Selection Toggle),
 *         TC-35-004 (Apply), TC-35-005 (Safe Mode Filter),
 *         TC-35-006 (Empty / Error States)
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, waitFor, fireEvent, act } from '../../testing/test-utils';
import RandomizerModal from './RandomizerModal';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, vars?: Record<string, unknown>) => {
      const translations: Record<string, string> = {
        'randomizer.title': 'Randomizer',
        'randomizer.safe_mode': 'Safe Mode',
        'randomizer.desc': 'Pick random mods',
        'randomizer.consulting': 'Consulting the RNG Gods...',
        'randomizer.deselect_all': 'Deselect All',
        'randomizer.select_all': 'Select All',
        'randomizer.reroll': 'Reroll All',
        'randomizer.roll': 'Roll Luck',
        'randomizer.no_eligible': 'No eligible character mods found',
        'randomizer.empty_desc': 'No results yet',
        'common:actions.close': 'Close',
      };

      if (key === 'randomizer.selection_status') {
        return `${String(vars?.selected ?? 0)} of ${String(vars?.total ?? 0)} selected`;
      }
      if (key === 'randomizer.apply') {
        return `Apply (${String(vars?.count ?? 0)})`;
      }
      if (key === 'randomizer.applying') {
        return 'Applying...';
      }

      return translations[key] ?? key;
    },
  }),
}));

import { invoke } from '@tauri-apps/api/core';

const mockProposals = [
  {
    object_id: 'obj-1',
    object_name: 'Hu Tao',
    mod_id: 'mod-a',
    name: 'Hu Tao Galaxy Skin',
    folder_path: 'E:/Mods/Hu Tao Galaxy',
  },
  {
    object_id: 'obj-2',
    object_name: 'Kazuha',
    mod_id: 'mod-b',
    name: 'Kazuha Samurai Skin',
    folder_path: 'E:/Mods/Kazuha Samurai',
  },
];

describe('RandomizerModal - TC-35', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Mock HTMLDialogElement.showModal() — not available in jsdom
    HTMLDialogElement.prototype.showModal = vi.fn();
    HTMLDialogElement.prototype.close = vi.fn();
  });

  describe('TC-35-001: Roll Luck - Automatic on Open', () => {
    it('invokes suggest_random_mods when modal opens', async () => {
      vi.mocked(invoke).mockResolvedValue(mockProposals);

      render(<RandomizerModal open={true} onClose={vi.fn()} gameId="g-1" />);

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('suggest_random_mods', {
          gameId: 'g-1',
          isSafe: true,
        });
      });
    });

    it('renders proposals after successful roll', async () => {
      vi.mocked(invoke).mockResolvedValue(mockProposals);

      render(<RandomizerModal open={true} onClose={vi.fn()} gameId="g-1" />);

      await waitFor(() => {
        expect(screen.getByText('Hu Tao Galaxy Skin')).toBeInTheDocument();
        expect(screen.getByText('Kazuha Samurai Skin')).toBeInTheDocument();
      });
    });

    it('shows "Consulting the RNG Gods..." loading state', async () => {
      // Delay response to capture loading state
      vi.mocked(invoke).mockImplementation(
        () =>
          new Promise((resolve) => {
            setTimeout(() => resolve(mockProposals), 500);
          }),
      );

      render(<RandomizerModal open={true} onClose={vi.fn()} gameId="g-1" />);

      expect(screen.getByText('Consulting the RNG Gods...')).toBeInTheDocument();
    });
  });

  describe('TC-35-002: Reroll', () => {
    it('invokes suggest_random_mods again when Reroll All is clicked', async () => {
      vi.mocked(invoke).mockResolvedValue(mockProposals);

      render(<RandomizerModal open={true} onClose={vi.fn()} gameId="g-1" />);

      await waitFor(() => {
        expect(screen.getByText('Hu Tao Galaxy Skin')).toBeInTheDocument();
      });

      // First call was on open; now click Reroll
      const rerollBtn = screen.getByText(/Reroll All/i, { selector: 'button' });
      await act(async () => {
        fireEvent.click(rerollBtn);
      });

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledTimes(2);
      });
    });
  });

  describe('TC-35-003: Selection Toggle', () => {
    it('all proposals are selected by default', async () => {
      vi.mocked(invoke).mockResolvedValue(mockProposals);

      render(<RandomizerModal open={true} onClose={vi.fn()} gameId="g-1" />);

      await waitFor(() => {
        expect(screen.getByText('2 of 2 selected')).toBeInTheDocument();
      });
    });

    it('toggles a single proposal selection on click', async () => {
      vi.mocked(invoke).mockResolvedValue(mockProposals);

      render(<RandomizerModal open={true} onClose={vi.fn()} gameId="g-1" />);

      await waitFor(() => {
        expect(screen.getByText('Hu Tao Galaxy Skin')).toBeInTheDocument();
      });

      // Click on the Hu Tao proposal to deselect
      const huTaoProposal = screen
        .getByText('Hu Tao Galaxy Skin')
        .closest('div[class*=flex]') as HTMLElement;
      await act(async () => {
        fireEvent.click(huTaoProposal);
      });

      expect(screen.getByText('1 of 2 selected')).toBeInTheDocument();
    });

    it('deselects all when "Deselect All" clicked', async () => {
      vi.mocked(invoke).mockResolvedValue(mockProposals);

      render(<RandomizerModal open={true} onClose={vi.fn()} gameId="g-1" />);

      await waitFor(() => {
        expect(screen.getByText(/Deselect All/i)).toBeInTheDocument();
      });

      await act(async () => {
        fireEvent.click(screen.getByText(/Deselect All/i));
      });

      expect(screen.getByText('0 of 2 selected')).toBeInTheDocument();
    });
  });

  describe('TC-35-004: Apply Selection', () => {
    it('calls execute_workspace_switch for each selected proposal', async () => {
      vi.mocked(invoke).mockResolvedValueOnce(mockProposals);
      vi.mocked(invoke).mockResolvedValue(undefined);

      const onClose = vi.fn();
      render(<RandomizerModal open={true} onClose={onClose} gameId="g-1" />);

      await waitFor(() => {
        expect(screen.getByText('Hu Tao Galaxy Skin')).toBeInTheDocument();
      });

      const applyBtn = screen.getByText(/Apply/i, { selector: 'button' });
      await act(async () => {
        fireEvent.click(applyBtn);
      });

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('execute_workspace_switch', {
          input: {
            game_id: 'g-1',
            target: {
              kind: 'mod_path',
              value: 'E:/Mods/Hu Tao Galaxy',
            },
            desired_enabled: true,
            resolution: 'enable_only_this',
            origin_surface: 'collections',
          },
        });
        expect(invoke).toHaveBeenCalledWith('execute_workspace_switch', {
          input: {
            game_id: 'g-1',
            target: {
              kind: 'mod_path',
              value: 'E:/Mods/Kazuha Samurai',
            },
            desired_enabled: true,
            resolution: 'enable_only_this',
            origin_surface: 'collections',
          },
        });
        expect(onClose).toHaveBeenCalled();
      });
    });

    it('disables apply button when nothing is selected', async () => {
      vi.mocked(invoke).mockResolvedValue(mockProposals);

      render(<RandomizerModal open={true} onClose={vi.fn()} gameId="g-1" />);

      await waitFor(() => {
        expect(screen.getByText(/Deselect All/i)).toBeInTheDocument();
      });

      // Deselect all
      const deselectBtn = screen.getByText(/Deselect All/i).closest('button')!;
      await act(async () => {
        fireEvent.click(deselectBtn);
      });
      await waitFor(() => {
        expect(screen.getByText('0 of 2 selected')).toBeInTheDocument();
      });

      const applyBtn = screen.getByText(/Apply/i, { selector: 'button' });
      expect(applyBtn).toBeDisabled();
    });
  });

  describe('TC-35-005: Safe Mode Filter', () => {
    it('defaults Safe Mode toggle to checked (true)', async () => {
      vi.mocked(invoke).mockResolvedValue([]);

      render(<RandomizerModal open={true} onClose={vi.fn()} gameId="g-1" />);
      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('suggest_random_mods', {
          gameId: 'g-1',
          isSafe: true,
        });
      });

      const toggle = screen.getByRole('checkbox', { hidden: true });
      expect(toggle).toBeChecked();
    });

    it('calls suggest_random_mods with isSafe=false when Safe Mode is unchecked', async () => {
      vi.mocked(invoke).mockResolvedValue([]);

      render(<RandomizerModal open={true} onClose={vi.fn()} gameId="g-1" />);

      // Wait for auto-roll to finish so toggle isn't disabled
      await waitFor(() => {
        expect(screen.queryByText(/Consulting the RNG Gods/i)).not.toBeInTheDocument();
      });

      const toggle = screen.getByRole('checkbox', { hidden: true });
      await act(async () => {
        fireEvent.click(toggle);
      });

      const rerollBtn = screen.getByText(/Roll Luck/i, { selector: 'button' });
      await act(async () => {
        fireEvent.click(rerollBtn);
      });

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('suggest_random_mods', {
          gameId: 'g-1',
          isSafe: false,
        });
      });
    });
  });

  describe('TC-35-006: Empty / Error States', () => {
    it('shows error alert when no eligible mods found', async () => {
      vi.mocked(invoke).mockResolvedValue([]);

      render(<RandomizerModal open={true} onClose={vi.fn()} gameId="g-1" />);

      await waitFor(() => {
        expect(screen.getByText(/No eligible character mods found/i)).toBeInTheDocument();
      });
    });

    it('shows error alert when invoke throws', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('Backend crashed'));

      render(<RandomizerModal open={true} onClose={vi.fn()} gameId="g-1" />);

      await waitFor(() => {
        expect(screen.getByText(/Backend crashed/i)).toBeInTheDocument();
      });
    });
  });
});
