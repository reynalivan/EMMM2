import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { GameType } from '@/entities/game/model/game';
import GameFormModal from './GameFormModal';

vi.mock('../../../shared/lib/hooks/useDialogSync', () => ({ useDialogSync: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));
vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

describe('GameFormModal', () => {
  it('closes only after the asynchronous save succeeds', async () => {
    let finishSave: (() => void) | undefined;
    const onSave = vi.fn(
      () =>
        new Promise<boolean>((resolve) => {
          finishSave = () => resolve(true);
        }),
    );
    const onClose = vi.fn();

    render(
      <GameFormModal
        isOpen
        onClose={onClose}
        onSave={onSave}
        initialData={{
          id: 'game-1',
          name: 'Genshin Impact',
          game_type: GameType.GIMI,
          mod_path: 'E:/GIMI/Mods',
          game_exe: 'E:/GIMI/GenshinImpact.exe',
          loader_exe: null,
          launch_args: null,
        }}
        existingModPaths={[]}
      />,
    );

    const submit = screen.getByTestId('game-form-submit');
    await waitFor(() => expect(submit).toBeEnabled());
    fireEvent.click(submit);
    await waitFor(() => expect(onSave).toHaveBeenCalledTimes(1));
    expect(onClose).not.toHaveBeenCalled();

    finishSave?.();
    await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1));
  });

  it('keeps the form open and reports an asynchronous save failure', async () => {
    const onClose = vi.fn();
    render(
      <GameFormModal
        isOpen
        onClose={onClose}
        onSave={vi.fn().mockRejectedValue(new Error('source apply failed'))}
        initialData={{
          id: 'game-1',
          name: 'Genshin Impact',
          game_type: GameType.GIMI,
          mod_path: 'E:/GIMI/Mods',
          game_exe: 'E:/GIMI/GenshinImpact.exe',
          loader_exe: null,
          launch_args: null,
        }}
        existingModPaths={[]}
      />,
    );

    const submit = screen.getByTestId('game-form-submit');
    await waitFor(() => expect(submit).toBeEnabled());
    fireEvent.click(submit);

    expect(await screen.findByText('source apply failed')).toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();
  });
});
