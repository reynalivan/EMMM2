import { act, render, screen, fireEvent, waitFor } from '../../../tests/testing/test-utils';
import { beforeEach, describe, it, expect, vi } from 'vitest';
import { AutoDetectResult } from './AutoDetectResult';
import type { GameConfig } from '@/entities/game';

const discovery = vi.hoisted(() => ({
  scan: vi.fn().mockResolvedValue(undefined),
  cancel: vi.fn(),
  validate: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@/features/game-executable-detection', () => ({
  useGameExecutableDiscovery: () => ({ ...discovery, pendingTypes: [] }),
  validateSelectedGameExecutables: discovery.validate,
  GameExecutableField: ({
    value,
    onChange,
    inputId,
    disabled,
  }: {
    value: string;
    onChange: (value: string, detected: boolean) => void;
    inputId: string;
    disabled: boolean;
  }) => (
    <div>
      <input
        aria-label={inputId}
        value={value}
        disabled={disabled}
        onChange={(event) => onChange(event.target.value, false)}
      />
      <button disabled={disabled} onClick={() => onChange('C:/Detected/Game.exe', true)}>
        Select {inputId}
      </button>
    </div>
  ),
}));

describe('AutoDetectResult (TC-03)', () => {
  const mockOnContinue = vi.fn();
  const mockOnAddMore = vi.fn();
  const mockOnRemoveGame = vi.fn();
  const mockOnGoBack = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    discovery.validate.mockResolvedValue(undefined);
  });

  it('renders games correctly', () => {
    const games = [
      { id: 'g1', name: 'Game 1', game_type: 'GIMI', mod_path: 'C:/Mods1', game_exe: 'C:/G1.exe' },
      { id: 'g2', name: 'Game 2', game_type: 'SRMI', mod_path: 'C:/Mods2', game_exe: 'C:/G2.exe' },
    ] as unknown as GameConfig[];

    render(
      <AutoDetectResult
        games={games}
        onConfirm={mockOnContinue}
        onAddMore={mockOnAddMore}
        onRemoveGame={mockOnRemoveGame}
        onBack={mockOnGoBack}
      />,
    );

    const title = screen.getByText('2 Games Found!');
    expect(title).toHaveClass('text-base-content');
    expect(title).not.toHaveClass('text-transparent');
    expect(screen.getByText('Game 1')).toBeInTheDocument();
    expect(screen.getByText('Game 2')).toBeInTheDocument();
    // Check for the new big checkmark icon (and the one in the button)
    expect(screen.getAllByTestId('icon-check')).toHaveLength(2);
  });

  it('handles empty state properly', () => {
    render(
      <AutoDetectResult
        games={[]}
        onConfirm={mockOnContinue}
        onAddMore={mockOnAddMore}
        onRemoveGame={mockOnRemoveGame}
        onBack={mockOnGoBack}
      />,
    );

    expect(screen.getByText('0 Games Found!')).toBeInTheDocument();
  });

  it('identifies XXMI-managed games without a standalone warning', () => {
    const games = [
      {
        id: 'wwmi',
        name: 'WWMI',
        game_type: 'WWMI',
        instance_path: 'E:/XXMI/WWMI',
        mod_path: 'E:/XXMI/WWMI/Mods',
        launch_mode: 'xxmi_managed',
        game_exe: null,
        loader_exe: null,
        xxmi_launcher_exe: 'E:/XXMI/Resources/Bin/XXMI Launcher.exe',
        launch_args: null,
      },
    ] as unknown as GameConfig[];

    render(
      <AutoDetectResult
        games={games}
        onConfirm={mockOnContinue}
        onAddMore={mockOnAddMore}
        onRemoveGame={mockOnRemoveGame}
        onBack={mockOnGoBack}
      />,
    );

    expect(screen.getByText('Managed by XXMI')).toBeInTheDocument();
    expect(screen.queryByText('Setup Warnings — you can still proceed')).not.toBeInTheDocument();
  });

  it('triggers callbacks', async () => {
    const games = [
      { id: 'g1', name: 'Game 1', game_type: 'GIMI', mod_path: 'C:/Mods', game_exe: 'C:/G.exe' },
    ] as unknown as GameConfig[];
    render(
      <AutoDetectResult
        games={games}
        onConfirm={mockOnContinue}
        onAddMore={mockOnAddMore}
        onRemoveGame={mockOnRemoveGame}
        onBack={mockOnGoBack}
      />,
    );

    fireEvent.click(screen.getByText('Back to Welcome'));
    expect(mockOnGoBack).toHaveBeenCalled();

    fireEvent.click(screen.getByText('Add Another'));
    expect(mockOnAddMore).toHaveBeenCalled();

    fireEvent.click(screen.getByText('Confirm'));
    await waitFor(() => expect(mockOnContinue).toHaveBeenCalled());

    const removeBtn = screen.getByTitle('Remove from detection');
    fireEvent.click(removeBtn);
    expect(mockOnRemoveGame).toHaveBeenCalledWith('g1');
  });

  const game = (id: string): GameConfig => ({
    id,
    name: id,
    game_type: 0,
    instance_path: `C:/XXMI/${id}`,
    mod_path: `C:/XXMI/${id}/Mods`,
    game_exe: null,
    loader_exe: null,
    xxmi_launcher_exe: null,
    launch_mode: 'xxmi_managed',
    launch_args: null,
  });

  const renderDraft = (games: GameConfig[]) =>
    render(
      <AutoDetectResult
        games={games}
        onConfirm={mockOnContinue}
        onAddMore={mockOnAddMore}
        onRemoveGame={mockOnRemoveGame}
        onBack={mockOnGoBack}
      />,
    );

  it('scans duplicate game types once and applies a selection to only its importer draft', async () => {
    renderDraft([game('first'), game('second')]);
    fireEvent.click(screen.getByText('Detect executable locations'));
    expect(discovery.scan).toHaveBeenCalledWith([0]);
    fireEvent.click(screen.getByText('Select onboarding-executable-second'));
    fireEvent.click(screen.getByText('Confirm'));
    await waitFor(() => expect(mockOnContinue).toHaveBeenCalled());
    expect(mockOnContinue.mock.calls[0][0]).toEqual([
      game('first'),
      { ...game('second'), game_exe: 'C:/Detected/Game.exe' },
    ]);
    expect(discovery.validate.mock.calls[0][1]).toEqual(
      new Map([['second', 'C:/Detected/Game.exe']]),
    );
  });

  it('keeps empty executable paths optional and cancels discovery before continuing', async () => {
    renderDraft([game('first')]);
    fireEvent.click(screen.getByText('Confirm'));
    await waitFor(() => expect(mockOnContinue).toHaveBeenCalledWith([game('first')]));
    expect(discovery.cancel).toHaveBeenCalled();
    expect(discovery.validate).toHaveBeenCalledWith([game('first')], new Map());
  });

  it('keeps the draft open if a selected executable disappeared', async () => {
    discovery.validate.mockRejectedValueOnce(new Error('Executable was moved'));
    renderDraft([game('first')]);
    fireEvent.click(screen.getByText('Select onboarding-executable-first'));
    fireEvent.click(screen.getByText('Confirm'));
    expect(await screen.findByRole('alert')).toHaveTextContent('Executable was moved');
    expect(mockOnContinue).not.toHaveBeenCalled();
    expect(screen.getByLabelText('onboarding-executable-first')).toHaveValue(
      'C:/Detected/Game.exe',
    );
    fireEvent.change(screen.getByLabelText('onboarding-executable-first'), {
      target: { value: 'C:/Custom.exe' },
    });
    fireEvent.click(screen.getByText('Confirm'));
    await waitFor(() => expect(mockOnContinue).toHaveBeenCalled());
    expect(discovery.validate.mock.calls[1][1]).toEqual(new Map());
  });

  it('ignores pending confirmation when leaving the result screen', async () => {
    let resolve: () => void = () => undefined;
    discovery.validate.mockReturnValueOnce(
      new Promise<void>((done) => {
        resolve = done;
      }),
    );
    renderDraft([game('first')]);
    fireEvent.click(screen.getByText('Confirm'));
    fireEvent.click(screen.getByText('Back to Welcome'));
    await act(async () => {
      resolve();
    });
    expect(mockOnGoBack).toHaveBeenCalledOnce();
    expect(mockOnContinue).not.toHaveBeenCalled();
  });
});
