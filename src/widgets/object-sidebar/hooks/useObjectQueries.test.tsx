import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { beforeEach, expect, it, vi } from 'vitest';
import { useMasterDb } from './useObjectQueries';

vi.unmock('@tanstack/react-query');
function wrapper({ children }: { children: ReactNode }) {
  return <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>;
}

const mocks = vi.hoisted(() => ({
  gameType: 0 as number | undefined,
  getMasterDb: vi.fn().mockResolvedValue([]),
}));
vi.mock('@/entities/game', () => ({
  useActiveGame: () => ({
    activeGame: mocks.gameType === undefined ? undefined : { game_type: mocks.gameType },
  }),
}));
vi.mock('../../../shared/api/tauri/bindings', () => ({ commands: mocks }));

beforeEach(() => {
  vi.clearAllMocks();
  mocks.gameType = 0;
});

it('loads the catalog for GIMI (game type zero)', async () => {
  renderHook(() => useMasterDb(), { wrapper });
  await waitFor(() => expect(mocks.getMasterDb).toHaveBeenCalledWith(0));
});

it('does not request a catalog without an active game', () => {
  mocks.gameType = undefined;
  renderHook(() => useMasterDb(), { wrapper });
  expect(mocks.getMasterDb).not.toHaveBeenCalled();
});
