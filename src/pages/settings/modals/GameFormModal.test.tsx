import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { GameType } from '@/entities/game';
import GameFormModal from './GameFormModal';

vi.mock('../../../shared/lib/hooks/useDialogSync', () => ({ useDialogSync: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));
vi.mock('react-i18next', async (importOriginal) => ({
  ...(await importOriginal<typeof import('react-i18next')>()),
  useTranslation: () => ({ t: (key: string) => key }),
}));

describe('GameFormModal', () => {
  it.each(['E:/GIMI/Mods', 'e:/gimi/mods', 'E:\\GIMI\\Mods\\'])(
    'blocks submission when the mods directory duplicates %s',
    async (modPath) => {
      const onSave = vi.fn().mockResolvedValue(true);
      const { container } = render(
        <GameFormModal
          isOpen
          onClose={vi.fn()}
          onSave={onSave}
          existingModPaths={['E:/GIMI/Mods']}
        />,
      );
      fireEvent.change(screen.getByPlaceholderText('games.form.name_placeholder'), {
        target: { value: 'GIMI' },
      });
      fireEvent.change(screen.getByPlaceholderText('games.form.exe_placeholder'), {
        target: { value: 'E:/GIMI/GenshinImpact.exe' },
      });
      fireEvent.change(screen.getByPlaceholderText('games.form.path_placeholder'), {
        target: { value: modPath },
      });
      fireEvent.submit(container.querySelector('form')!);

      expect(await screen.findByText('games.form.validation.path_duplicate')).toBeInTheDocument();
      expect(screen.getByTestId('game-form-submit')).toBeDisabled();
      expect(onSave).not.toHaveBeenCalled();
    },
  );

  it('allows a different mods directory with the same folder name', async () => {
    const onSave = vi.fn().mockResolvedValue(true);
    render(
      <GameFormModal
        isOpen
        onClose={vi.fn()}
        onSave={onSave}
        existingModPaths={['E:/GIMI/Mods']}
      />,
    );
    fireEvent.change(screen.getByPlaceholderText('games.form.name_placeholder'), {
      target: { value: 'GIMI' },
    });
    fireEvent.change(screen.getByPlaceholderText('games.form.exe_placeholder'), {
      target: { value: 'D:/GIMI/GenshinImpact.exe' },
    });
    fireEvent.change(screen.getByPlaceholderText('games.form.path_placeholder'), {
      target: { value: 'D:/GIMI/Mods' },
    });
    const submit = screen.getByTestId('game-form-submit');
    await waitFor(() => expect(submit).toBeEnabled());
    fireEvent.click(submit);

    await waitFor(() => expect(onSave).toHaveBeenCalledOnce());
    expect(onSave).toHaveBeenCalledWith(expect.objectContaining({ mod_path: 'D:/GIMI/Mods' }));
  });

  it('shows a migration status while a mods directory change is running', () => {
    render(
      <GameFormModal
        isOpen
        onClose={vi.fn()}
        onSave={vi.fn().mockResolvedValue(true)}
        existingModPaths={[]}
        isSourceMigrationPending
      />,
    );

    expect(screen.getByRole('status', { hidden: true })).toHaveTextContent(
      'games.source_change_progress',
    );
  });

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
          name: 'GIMI',
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
          name: 'GIMI',
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

  it('uses the shared XXMI launcher for the managed launch mode', async () => {
    const onSave = vi.fn().mockResolvedValue(true);
    render(<GameFormModal isOpen onClose={vi.fn()} onSave={onSave} existingModPaths={[]} />);

    const launchMode = document.querySelector<HTMLSelectElement>('select[name="launch_mode"]');
    expect(launchMode).not.toBeNull();
    fireEvent.change(launchMode!, {
      target: { value: 'xxmi_managed' },
    });
    fireEvent.change(screen.getByPlaceholderText('games.form.path_placeholder'), {
      target: { value: 'E:/XXMI/WWMI/Mods' },
    });
    fireEvent.change(screen.getByPlaceholderText('games.form.xxmi_placeholder'), {
      target: { value: 'E:/XXMI/Resources/Bin/XXMI Launcher.exe' },
    });
    fireEvent.change(screen.getByPlaceholderText('games.form.name_placeholder'), {
      target: { value: 'WWMI' },
    });

    const submit = screen.getByTestId('game-form-submit');
    await waitFor(() => expect(submit).toBeEnabled());
    fireEvent.click(submit);

    await waitFor(() => expect(onSave).toHaveBeenCalledTimes(1));
    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({
        launch_mode: 'xxmi_managed',
        game_exe: null,
        loader_exe: null,
        xxmi_launcher_exe: 'E:/XXMI/Resources/Bin/XXMI Launcher.exe',
      }),
    );
  });
});
