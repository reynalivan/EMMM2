import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useFileDrop } from './useFileDrop';
import { listen } from '@tauri-apps/api/event';
import { toast } from '../stores/useToastStore';

// Mock dependecies
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

vi.mock('../stores/useToastStore', () => ({
  toast: {
    error: vi.fn(),
    warning: vi.fn(),
  },
}));

describe('useFileDrop Hook (TC-23)', () => {
  let mockEventHandlers: Record<string, (event: unknown) => void> = {};

  beforeEach(() => {
    vi.clearAllMocks();
    mockEventHandlers = {};
    (listen as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (event: string, callback: (e: unknown) => void) => {
        mockEventHandlers[event] = callback;
        return Promise.resolve(vi.fn()); // Mock unlisten function
      },
    );

    Object.defineProperty(window, 'devicePixelRatio', {
      writable: true,
      value: 1,
    });
  });

  it('TC-23-001: Drop Overlay Trigger (Registers listeners and updates state)', async () => {
    const onDrop = vi.fn();
    const onDragStateChange = vi.fn();

    const { result, unmount } = renderHook(() =>
      useFileDrop({
        onDrop,
        onDragStateChange,
        enabled: true,
      }),
    );

    // Wait for async listen registrations
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    // Simulate drag enter
    act(() => {
      mockEventHandlers['tauri://drag-enter']({
        payload: { position: { x: 100, y: 100 }, paths: [] },
      });
    });

    expect(result.current.isDragging).toBe(true);
    expect(result.current.dragPosition).toEqual({ x: 100, y: 100 });
    expect(onDragStateChange).toHaveBeenCalledWith(true);

    // Simulate drag leave
    act(() => {
      mockEventHandlers['tauri://drag-leave']({});
    });

    expect(result.current.isDragging).toBe(false);
    expect(onDragStateChange).toHaveBeenCalledWith(false);

    unmount();
  });

  it('TC-23-002: Single Archive Pipeline (Valid payload)', async () => {
    const onDrop = vi.fn();

    renderHook(() =>
      useFileDrop({
        onDrop,
        enabled: true,
      }),
    );

    // Wait for async listen registrations
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    act(() => {
      mockEventHandlers['tauri://drag-drop']({
        payload: {
          paths: ['C:\\Mods\\ValidArchive.zip'],
          position: { x: 10, y: 20 },
        },
      });
    });

    expect(toast.error).not.toHaveBeenCalled();
    expect(onDrop).toHaveBeenCalledWith(['C:\\Mods\\ValidArchive.zip'], { x: 10, y: 20 });
  });

  it('TC-23-004: File Type Whitelisting (Unsupported files rejected entirely)', async () => {
    const onDrop = vi.fn();

    renderHook(() =>
      useFileDrop({
        onDrop,
        enabled: true,
      }),
    );

    // Wait for listen registrations
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    act(() => {
      mockEventHandlers['tauri://drag-drop']({
        payload: {
          paths: ['C:\\Mods\\installer.exe', 'C:\\Docs\\readme.pdf'],
          position: { x: 0, y: 0 },
        },
      });
    });

    // Validates rejection logic
    expect(toast.error).toHaveBeenCalledWith(expect.stringContaining('Unsupported file type'));
    expect(onDrop).not.toHaveBeenCalled();
  });

  it('TC-23-004: Mixed payload drops skip unsupported elements but process valid ones', async () => {
    const onDrop = vi.fn();

    renderHook(() =>
      useFileDrop({
        onDrop,
        enabled: true,
      }),
    );

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    act(() => {
      mockEventHandlers['tauri://drag-drop']({
        payload: {
          // mix of valid and invalid
          paths: ['C:\\Mods\\ValidMod.rar', 'C:\\Mods\\bad.exe'],
          position: { x: 0, y: 0 },
        },
      });
    });

    expect(toast.error).not.toHaveBeenCalled(); // Error only fires if all are invalid
    expect(toast.warning).toHaveBeenCalledWith('Skipped 1 unsupported file(s).');

    // Calls onDrop with ALL paths; useFolderGridActions handles the specific routing based on context
    expect(onDrop).toHaveBeenCalledWith(['C:\\Mods\\ValidMod.rar', 'C:\\Mods\\bad.exe'], {
      x: 0,
      y: 0,
    });
  });
});
