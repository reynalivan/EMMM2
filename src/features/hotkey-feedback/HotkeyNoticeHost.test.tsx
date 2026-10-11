import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Event, EventCallback } from '@tauri-apps/api/event';
import type { HotkeyNotice } from '@/shared/api/tauri/bindings.gen';
import { hotkeyNoticeFixture } from '@/tests/testing/hotkeyNoticeFixture';
import { HotkeyNoticeHost } from './HotkeyNoticeHost';
import { useHotkeyNoticeStore } from './store';

const bridge = vi.hoisted(() => ({
  listen: vi.fn(),
  get: vi.fn(),
  unlisten: vi.fn(),
  settings: {
    active_game_id: 'game-a',
    games: [{ id: 'game-a', mod_path: 'C:\\fixture\\Mods' }],
    hotkeys: {
      enabled: true,
      safe_mode: 'Alt+F5',
      next_preset: 'Ctrl+F5',
      prev_preset: 'Shift+F5',
      toggle_overlay: 'F7',
    },
    keyviewer: { enabled: true },
  },
  runtime: {} as Record<string, { game_id: string; generation: number; phase: string }>,
  requestedGameId: null as string | null,
}));
vi.mock('@tauri-apps/api/event', () => ({ listen: bridge.listen }));
vi.mock('@/shared/api/tauri/bindings', () => ({ commands: { getHotkeyNotice: bridge.get } }));
vi.mock('@/entities/settings', () => ({ useSettings: () => ({ settings: bridge.settings }) }));
vi.mock('@/app/store', () => ({
  useAppStore: (
    select: (value: {
      runtimeSyncByGame: typeof bridge.runtime;
      requestedGameId: string | null;
    }) => unknown,
  ) => select({ runtimeSyncByGame: bridge.runtime, requestedGameId: bridge.requestedGameId }),
}));

let receive: EventCallback<HotkeyNotice>;
async function deliver(notice: HotkeyNotice) {
  await act(async () =>
    receive({ event: 'hotkey:notice', id: 1, payload: notice } satisfies Event<HotkeyNotice>),
  );
}

describe('global shell hotkey feedback', () => {
  afterEach(() => vi.useRealTimers());
  beforeEach(() => {
    vi.clearAllMocks();
    bridge.runtime = {};
    bridge.requestedGameId = null;
    bridge.settings.active_game_id = 'game-a';
    useHotkeyNoticeStore.setState({ notice: null, lastSequence: 0 });
    bridge.get.mockResolvedValue(null);
    bridge.listen.mockImplementation(
      async (_event: string, callback: EventCallback<HotkeyNotice>) => {
        receive = callback;
        return bridge.unlisten;
      },
    );
  });

  it('shows the new collection and all helpers without taking input focus', async () => {
    render(
      <>
        <input aria-label="Existing control" />
        <HotkeyNoticeHost />
      </>,
    );
    screen.getByRole('textbox').focus();
    await waitFor(() => expect(bridge.get).toHaveBeenCalledOnce());
    await deliver(hotkeyNoticeFixture());
    expect(screen.getByText('Combat')).toHaveClass('text-3xl');
    expect(screen.getByText('From Exploration')).toBeInTheDocument();
    expect(screen.getByText('Hotkey helpers')).toBeInTheDocument();
    expect(screen.getByText('Alt+F5')).toBeInTheDocument();
    expect(screen.getByText('Ctrl+F5')).toBeInTheDocument();
    expect(screen.getByText('F7')).toBeInTheDocument();
    expect(screen.getByRole('textbox')).toHaveFocus();
    fireEvent.click(screen.getByRole('button', { name: 'Dismiss hotkey notification' }));
    expect(screen.queryByText('Combat')).not.toBeInTheDocument();
  });

  it('rejects an old async snapshot after a newer event and does not resurrect dismissed feedback', async () => {
    let resolveSnapshot!: (notice: HotkeyNotice) => void;
    bridge.get.mockReturnValue(
      new Promise<HotkeyNotice>((resolve) => {
        resolveSnapshot = resolve;
      }),
    );
    render(<HotkeyNoticeHost />);
    await waitFor(() => expect(bridge.get).toHaveBeenCalledOnce());
    await deliver(hotkeyNoticeFixture({ id: 'new', sequence: 2, collection: 'New' }));
    await act(async () => resolveSnapshot(hotkeyNoticeFixture()));
    expect(screen.getByText('New')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Dismiss hotkey notification' }));
    await deliver(hotkeyNoticeFixture({ sequence: 2 }));
    expect(screen.queryByText('Combat')).not.toBeInTheDocument();
  });

  it('ignores another game and clears feedback on game selection changes', async () => {
    const { rerender } = render(<HotkeyNoticeHost />);
    await waitFor(() => expect(bridge.get).toHaveBeenCalledOnce());
    await deliver(hotkeyNoticeFixture({ game_id: 'game-b' }));
    expect(screen.queryByText('Combat')).not.toBeInTheDocument();
    await deliver(hotkeyNoticeFixture({ sequence: 2 }));
    expect(screen.getByText('Combat')).toBeInTheDocument();
    bridge.settings.active_game_id = 'game-b';
    rerender(<HotkeyNoticeHost />);
    expect(screen.queryByText('Combat')).not.toBeInTheDocument();
    expect(bridge.unlisten).toHaveBeenCalledOnce();
  });

  it('does not let unrelated runtime generations certify this notice', async () => {
    const { rerender } = render(<HotkeyNoticeHost />);
    await waitFor(() => expect(bridge.get).toHaveBeenCalledOnce());
    await deliver(hotkeyNoticeFixture());
    bridge.runtime['game-a'] = { game_id: 'game-a', generation: 1, phase: 'succeeded' };
    rerender(<HotkeyNoticeHost />);
    expect(screen.getByText('Mod changes saved. Runtime update pending.')).toBeInTheDocument();
    bridge.runtime['game-a'] = { game_id: 'game-a', generation: 2, phase: 'needs_manual_reload' };
    rerender(<HotkeyNoticeHost />);
    expect(
      screen.getByText('Runtime files updated. Reload in game to apply them.'),
    ).toBeInTheDocument();
  });

  it('clears the old receipt immediately during a requested game transition', async () => {
    const { rerender } = render(<HotkeyNoticeHost />);
    await waitFor(() => expect(bridge.get).toHaveBeenCalledOnce());
    await deliver(hotkeyNoticeFixture());
    bridge.requestedGameId = 'game-b';
    rerender(<HotkeyNoticeHost />);
    expect(screen.queryByText('Combat')).not.toBeInTheDocument();
    expect(useHotkeyNoticeStore.getState().notice).toBeNull();
  });

  it('unsubscribes when async listener setup finishes after unmount', async () => {
    let finishListen!: (unlisten: () => void) => void;
    bridge.listen.mockReturnValue(
      new Promise<() => void>((resolve) => {
        finishListen = resolve;
      }),
    );
    const { unmount } = render(<HotkeyNoticeHost />);
    unmount();
    await act(async () => finishListen(bridge.unlisten));
    expect(bridge.unlisten).toHaveBeenCalledOnce();
    expect(bridge.get).not.toHaveBeenCalled();
  });

  it('keeps live feedback available after a snapshot read failure', async () => {
    bridge.get.mockRejectedValue(new Error('Snapshot unavailable'));
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const error = vi.spyOn(console, 'error').mockImplementation(() => {});
    render(<HotkeyNoticeHost />);
    await waitFor(() => expect(bridge.get).toHaveBeenCalledOnce());
    await deliver(hotkeyNoticeFixture());
    expect(screen.getByText('Combat')).toBeInTheDocument();
    expect(warn).toHaveBeenCalledOnce();
    expect(error).not.toHaveBeenCalled();
    warn.mockRestore();
    error.mockRestore();
  });

  it('keeps the successor visible when the old expiry passes, then expires it', async () => {
    vi.useFakeTimers();
    render(<HotkeyNoticeHost />);
    await act(async () => {});
    await deliver(hotkeyNoticeFixture());
    await act(async () => vi.advanceTimersByTime(1000));
    await deliver(hotkeyNoticeFixture({ id: 'successor', sequence: 2, collection: 'New' }));
    await act(async () => vi.advanceTimersByTime(4000));
    expect(screen.getByText('New')).toBeInTheDocument();
    await act(async () => vi.advanceTimersByTime(1000));
    expect(screen.queryByText('New')).not.toBeInTheDocument();
  });

  it('leaves Escape to open modal dialogs but allows dismissal otherwise', async () => {
    render(<HotkeyNoticeHost />);
    await waitFor(() => expect(bridge.get).toHaveBeenCalledOnce());
    await deliver(hotkeyNoticeFixture());
    const modal = document.createElement('dialog');
    modal.setAttribute('open', '');
    document.body.append(modal);
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.getByText('Combat')).toBeInTheDocument();
    modal.remove();
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByText('Combat')).not.toBeInTheDocument();
  });
});
