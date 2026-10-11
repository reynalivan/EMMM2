import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { GameType } from '@/entities/game';
import type { GameConfig } from '@/entities/game';
import { open } from '@tauri-apps/plugin-dialog';
import GameFormModal from './GameFormModal';

vi.mock('../../../shared/lib/hooks/useDialogSync', () => ({ useDialogSync: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));
vi.mock('react-i18next', async (importOriginal) => ({
  ...(await importOriginal<typeof import('react-i18next')>()),
  useTranslation: () => ({ t: (key: string) => key }),
}));

const gameA: GameConfig = {
  id: 'game-a',
  name: 'Game A',
  game_type: GameType.GIMI,
  mod_path: 'E:/GameA/Mods',
  game_exe: 'E:/GameA/Game.exe',
  loader_exe: null,
  launch_args: null,
};
const gameB: GameConfig = {
  ...gameA,
  id: 'game-b',
  name: 'Game B',
  mod_path: 'E:/GameB/Mods',
  game_exe: 'E:/GameB/Game.exe',
};

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

describe('GameFormModal', () => {
  beforeEach(() => {
    vi.mocked(open).mockReset();
  });

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

  it('prevents native dialog cancellation while a save is pending', async () => {
    const save = deferred<boolean>();
    const onSave = vi.fn(() => save.promise);
    const onClose = vi.fn();
    const view = render(
      <GameFormModal
        isOpen
        onClose={onClose}
        onSave={onSave}
        initialData={gameA}
        existingModPaths={[]}
      />,
    );
    await waitFor(() => expect(screen.getByTestId('game-form-submit')).toBeEnabled());
    fireEvent.click(screen.getByTestId('game-form-submit'));
    await waitFor(() => expect(onSave).toHaveBeenCalledOnce());

    const cancel = new Event('cancel', { cancelable: true });
    fireEvent(view.container.querySelector('dialog')!, cancel);
    try {
      expect(cancel.defaultPrevented).toBe(true);
      expect(onClose).not.toHaveBeenCalled();
    } finally {
      await act(async () => save.resolve(false));
    }
  });

  it('prevents native cancellation during source migration', async () => {
    const view = render(
      <GameFormModal
        isOpen
        onClose={vi.fn()}
        onSave={vi.fn()}
        initialData={gameA}
        existingModPaths={[]}
        isSourceMigrationPending
      />,
    );
    const cancel = new Event('cancel', { cancelable: true });
    fireEvent(view.container.querySelector('dialog')!, cancel);
    expect(cancel.defaultPrevented).toBe(true);
    await waitFor(() => expect(screen.getByTestId('game-form-submit')).toBeDisabled());
  });

  it.each(['success', 'failure'] as const)(
    'keeps the replacement game draft when the previous save finishes with %s',
    async (result) => {
      const save = deferred<boolean>();
      const onSave = vi.fn(() => save.promise);
      const onClose = vi.fn();
      const tree = (game: GameConfig, isOpen = true) => (
        <GameFormModal
          isOpen={isOpen}
          onClose={onClose}
          onSave={onSave}
          initialData={game}
          existingModPaths={[]}
        />
      );
      const view = render(tree(gameA));
      await waitFor(() => expect(screen.getByTestId('game-form-submit')).toBeEnabled());
      fireEvent.click(screen.getByTestId('game-form-submit'));
      await waitFor(() => expect(onSave).toHaveBeenCalledOnce());
      expect(onSave).toHaveBeenCalledWith(
        expect.objectContaining({ id: gameA.id, name: gameA.name }),
      );

      view.rerender(tree(gameA, false));
      view.rerender(tree(gameB));
      fireEvent.change(screen.getByPlaceholderText('games.form.name_placeholder'), {
        target: { value: 'Game B draft' },
      });
      await act(async () => {
        if (result === 'success') save.resolve(true);
        else save.reject(new Error('Game A failed'));
      });

      expect(onClose).not.toHaveBeenCalled();
      expect(screen.getByPlaceholderText('games.form.name_placeholder')).toHaveValue(
        'Game B draft',
      );
      expect(screen.queryByText('Game A failed')).not.toBeInTheDocument();
      await waitFor(() => expect(screen.getByTestId('game-form-submit')).toBeEnabled());
    },
  );

  it('keeps a new save pending when an older game save completes', async () => {
    const saveA = deferred<boolean>();
    const saveB = deferred<boolean>();
    const onSave = vi.fn().mockReturnValueOnce(saveA.promise).mockReturnValueOnce(saveB.promise);
    const onClose = vi.fn();
    const tree = (game: GameConfig) => (
      <GameFormModal
        isOpen
        onClose={onClose}
        onSave={onSave}
        initialData={game}
        existingModPaths={[]}
      />
    );
    const view = render(tree(gameA));
    await waitFor(() => expect(screen.getByTestId('game-form-submit')).toBeEnabled());
    fireEvent.click(screen.getByTestId('game-form-submit'));
    await waitFor(() => expect(onSave).toHaveBeenCalledTimes(1));
    view.rerender(tree(gameB));
    await waitFor(() => expect(screen.getByTestId('game-form-submit')).toBeEnabled());
    fireEvent.click(screen.getByTestId('game-form-submit'));
    await waitFor(() => expect(onSave).toHaveBeenCalledTimes(2));

    await act(async () => saveA.resolve(true));
    try {
      expect(onClose).not.toHaveBeenCalled();
      expect(screen.getByTestId('game-form-submit')).toBeDisabled();
      const cancel = new Event('cancel', { cancelable: true });
      fireEvent(view.container.querySelector('dialog')!, cancel);
      expect(cancel.defaultPrevented).toBe(true);
    } finally {
      await act(async () => saveB.resolve(true));
    }
    expect(onClose).toHaveBeenCalledOnce();
  });

  it.each([
    ['games.form.path_placeholder', 'E:/OldPicker/Mods', gameB.mod_path],
    ['games.form.exe_placeholder', 'E:/OldPicker/Game.exe', gameB.game_exe],
  ])(
    'ignores a late picker result for %s after replacing the game',
    async (placeholder, selected, expected) => {
      const picker = deferred<string | null>();
      vi.mocked(open).mockReturnValueOnce(picker.promise);
      const tree = (game: GameConfig) => (
        <GameFormModal
          isOpen
          onClose={vi.fn()}
          onSave={vi.fn()}
          initialData={game}
          existingModPaths={[]}
        />
      );
      const view = render(tree(gameA));
      fireEvent.click(
        screen.getByPlaceholderText(placeholder).parentElement!.querySelector('button')!,
      );
      expect(open).toHaveBeenCalledOnce();
      view.rerender(tree(gameB));
      expect(screen.getByPlaceholderText(placeholder)).toHaveValue(expected);
      await act(async () => picker.resolve(selected));
      expect(screen.getByPlaceholderText(placeholder)).toHaveValue(expected);
    },
  );
});
