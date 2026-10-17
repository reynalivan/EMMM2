import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { commands } from '@/shared/api/tauri/bindings';
import { GameExecutableField } from './GameExecutableField';
import { validateSelectedGameExecutables } from './validateSelection';

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));
vi.mock('@/shared/api/tauri/bindings', () => ({
  commands: { discoverGameExecutables: vi.fn(), validateGameExecutable: vi.fn() },
}));

const candidate = {
  game_type: 4,
  executable_path: 'E:/Endfield/Endfield.exe',
  install_root: 'E:/Endfield',
  process_executable_path: 'E:/Endfield/Endfield.exe',
  sources: ['xxmi_config' as const],
  region: null,
  channel: null,
};

function Harness({ gameType = 4 }: { gameType?: number }) {
  const [value, setValue] = useState('');
  return <GameExecutableField gameType={gameType} value={value} onChange={setValue} />;
}

describe('GameExecutableField', () => {
  beforeEach(() => {
    vi.mocked(commands.discoverGameExecutables).mockReset();
    vi.mocked(commands.validateGameExecutable).mockReset();
    vi.mocked(open).mockReset();
    vi.mocked(commands.discoverGameExecutables).mockResolvedValue({
      candidates: [candidate],
      issues: [],
      complete: true,
    });
    vi.mocked(commands.validateGameExecutable).mockResolvedValue(candidate);
  });

  it('requires explicit selection even for one Endfield candidate', async () => {
    render(<Harness />);
    expect(commands.discoverGameExecutables).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Detect' }));
    const use = await screen.findByRole('button', { name: 'Use this location' });
    expect(screen.getByRole('textbox')).toHaveValue('');
    fireEvent.click(use);
    await waitFor(() => expect(screen.getByRole('textbox')).toHaveValue(candidate.executable_path));
    expect(commands.validateGameExecutable).toHaveBeenCalledWith({
      game_type: 4,
      executable_path: candidate.executable_path,
    });
  });

  it('does not apply validation after a newer manual edit', async () => {
    let complete!: (value: typeof candidate) => void;
    vi.mocked(commands.validateGameExecutable).mockReturnValueOnce(
      new Promise((resolve) => {
        complete = resolve;
      }),
    );
    render(<Harness />);
    fireEvent.click(screen.getByRole('button', { name: 'Detect' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Use this location' }));
    fireEvent.change(screen.getByRole('textbox'), { target: { value: 'E:/Manual/custom.exe' } });
    await act(async () => complete(candidate));
    expect(screen.getByRole('textbox')).toHaveValue('E:/Manual/custom.exe');
  });

  it('ignores a scan completing after the selected game type changes', async () => {
    let complete!: (value: {
      candidates: (typeof candidate)[];
      issues: [];
      complete: boolean;
    }) => void;
    vi.mocked(commands.discoverGameExecutables).mockReturnValueOnce(
      new Promise((resolve) => {
        complete = resolve;
      }),
    );
    const view = render(<Harness />);
    fireEvent.click(screen.getByRole('button', { name: 'Detect' }));
    view.rerender(<Harness gameType={0} />);
    await act(async () => complete({ candidates: [candidate], issues: [], complete: true }));
    expect(screen.queryByRole('button', { name: 'Use this location' })).not.toBeInTheDocument();
    expect(screen.getByRole('textbox')).toHaveValue('');
  });

  it('keeps Browse available when discovery returns no candidates', async () => {
    vi.mocked(commands.discoverGameExecutables).mockResolvedValueOnce({
      candidates: [],
      issues: [],
      complete: true,
    });
    vi.mocked(open).mockResolvedValueOnce('E:/Manual/custom.exe');
    render(<Harness />);
    fireEvent.click(screen.getByRole('button', { name: 'Detect' }));
    expect(
      await screen.findByText('No executable found. Choose a file or try again.'),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Browse' }));
    await waitFor(() => expect(screen.getByRole('textbox')).toHaveValue('E:/Manual/custom.exe'));
  });

  it('revalidates discovered selections before saving and preserves manual paths', async () => {
    await validateSelectedGameExecutables(
      [
        { id: 'a', game_type: 4, game_exe: candidate.executable_path },
        { id: 'b', game_type: 0, game_exe: 'E:/Manual/custom.exe' },
      ],
      new Map([['a', candidate.executable_path]]),
    );
    expect(commands.validateGameExecutable).toHaveBeenCalledTimes(1);
    vi.mocked(commands.validateGameExecutable).mockRejectedValueOnce(new Error('Executable moved'));
    await expect(
      validateSelectedGameExecutables(
        [{ id: 'a', game_type: 4, game_exe: candidate.executable_path }],
        new Map([['a', candidate.executable_path]]),
      ),
    ).rejects.toThrow('Executable moved');
  });

  it('discards a scan that completes after cancellation', async () => {
    let complete!: (value: {
      candidates: (typeof candidate)[];
      issues: [];
      complete: boolean;
    }) => void;
    vi.mocked(commands.discoverGameExecutables).mockReturnValueOnce(
      new Promise((resolve) => {
        complete = resolve;
      }),
    );
    render(<Harness />);
    fireEvent.click(screen.getByRole('button', { name: 'Detect' }));
    fireEvent.click(screen.getByRole('button', { name: 'Cancel detection' }));
    await act(async () => complete({ candidates: [candidate], issues: [], complete: true }));
    expect(screen.queryByRole('button', { name: 'Use this location' })).not.toBeInTheDocument();
    expect(screen.getByRole('textbox')).toHaveValue('');
  });

  it('keeps valid candidates visible when another source fails', async () => {
    vi.mocked(commands.discoverGameExecutables).mockResolvedValueOnce({
      candidates: [candidate],
      complete: false,
      issues: [{ source: 'hoyoplay_config', code: 'malformed_config' }],
    });
    render(<Harness />);
    fireEvent.click(screen.getByRole('button', { name: 'Detect' }));
    expect(await screen.findByRole('button', { name: 'Use this location' })).toBeEnabled();
    expect(
      screen.getByText('HoYoPlay configuration: Configuration format could not be read'),
    ).toBeInTheDocument();
  });

  it('retires pending selection when its form begins confirmation', async () => {
    let complete!: (value: typeof candidate) => void;
    vi.mocked(commands.validateGameExecutable).mockReturnValueOnce(
      new Promise((resolve) => {
        complete = resolve;
      }),
    );
    const change = vi.fn();
    const view = render(<GameExecutableField gameType={4} value="" onChange={change} />);
    fireEvent.click(screen.getByRole('button', { name: 'Detect' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Use this location' }));
    view.rerender(<GameExecutableField gameType={4} value="" onChange={change} disabled />);
    await act(async () => complete(candidate));
    expect(change).not.toHaveBeenCalled();
  });

  it('pages large discovery results without dropping available locations', async () => {
    vi.mocked(commands.discoverGameExecutables).mockResolvedValueOnce({
      candidates: Array.from({ length: 51 }, (_, index) => ({
        ...candidate,
        executable_path: `E:/Install${index}/Endfield.exe`,
      })),
      complete: true,
      issues: [],
    });
    render(<Harness />);
    fireEvent.click(screen.getByRole('button', { name: 'Detect' }));
    await screen.findByRole('button', { name: 'Next' });
    expect(screen.getAllByRole('button', { name: 'Use this location' })).toHaveLength(50);
    fireEvent.click(screen.getByRole('button', { name: 'Next' }));
    expect(screen.getAllByRole('button', { name: 'Use this location' })).toHaveLength(1);
    expect(screen.getByText('E:/Install50/Endfield.exe')).toBeInTheDocument();
  });
});
