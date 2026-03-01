import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render } from '@testing-library/react';
import { ExternalChangeHandler } from './ExternalChangeHandler';
import { toast } from '../../stores/useToastStore';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';

// Mock dependecies
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

let mockListener: (event: unknown) => void;

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockImplementation((event: string, callback: (event: unknown) => void) => {
    if (event === 'mod_watch:event') {
      mockListener = callback;
    }
    return Promise.resolve(vi.fn()); // Returns unlisten function
  }),
}));

vi.mock('../../stores/useToastStore', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
  },
}));

vi.mock('../../hooks/useActiveGame', () => ({
  useActiveGame: () => ({
    activeGame: {
      id: 'genshin',
      name: 'Genshin Impact',
      game_type: 'Genshin',
      mod_path: 'C:/Genshin/Mods',
    },
  }),
}));

const queryClient = new QueryClient();
const wrapper = ({ children }: { children: React.ReactNode }) => (
  <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
);

describe('ExternalChangeHandler (TC-28 File Watcher)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('TC-28-001: File Create triggers auto-refresh and toast info', () => {
    render(<ExternalChangeHandler />, { wrapper });

    // Simulate event
    mockListener({
      payload: {
        type: 'Created',
        path: 'C:\\Genshin\\Mods\\NewCharacter',
      },
    });

    expect(toast.info).toHaveBeenCalledWith(
      expect.stringContaining('"NewCharacter" was added externally. View refreshed.'),
    );
  });

  it('TC-28-002: File Delete triggers auto-refresh and toast warning', () => {
    render(<ExternalChangeHandler />, { wrapper });

    // Simulate event
    mockListener({
      payload: {
        type: 'Removed',
        path: 'C:/Genshin/Mods/OldCharacter',
      },
    });

    expect(toast.warning).toHaveBeenCalledWith(
      expect.stringContaining('"OldCharacter" was removed externally. View refreshed.'),
    );
  });

  it('TC-28-003: File Rename triggers auto-refresh and toast info', () => {
    render(<ExternalChangeHandler />, { wrapper });

    // Simulate event
    mockListener({
      payload: {
        type: 'Renamed',
        from: 'C:/Genshin/Mods/OldName',
        to: 'C:/Genshin/Mods/NewName',
      },
    });

    expect(toast.info).toHaveBeenCalledWith(
      expect.stringContaining('"OldName" renamed to "NewName" externally. View refreshed.'),
    );
  });

  it('TC-28-004: Nested file changes DO NOT trigger toasts', () => {
    render(<ExternalChangeHandler />, { wrapper });

    // Simulate event for nested INI file (should not trigger toast)
    mockListener({
      payload: {
        type: 'Created',
        path: 'C:/Genshin/Mods/KeqingMod/mod.ini',
      },
    });

    // Sub-folder changes > 1 depth should not trigger toast either
    mockListener({
      payload: {
        type: 'Created',
        path: 'C:/Genshin/Mods/Characters/KeqingMod/textures',
      },
    });

    expect(toast.info).not.toHaveBeenCalled();
    expect(toast.warning).not.toHaveBeenCalled();
  });
});
