import { renderHook, waitFor } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { useMetadataSyncQuery, useAssetFetch } from './useMetadataSync';
import { invoke } from '@tauri-apps/api/core';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return ({ children }: { children: React.ReactNode }) =>
    React.createElement(QueryClientProvider, { client: queryClient }, children);
};

describe('useMetadataSync', () => {
  describe('useMetadataSyncQuery', () => {
    it('triggers metadata update check via invoke', async () => {
      vi.mocked(invoke).mockResolvedValue({ updated: true, version: 2 });
      const { result } = renderHook(() => useMetadataSyncQuery(), { wrapper: createWrapper() });

      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true);
      });

      expect(invoke).toHaveBeenCalledWith('check_metadata_update');
      expect(result.current.data).toEqual({ updated: true, version: 2 });
    });
  });

  describe('useAssetFetch', () => {
    it('calls fetch_missing_asset correctly', async () => {
      const mockMutateFn = vi.mocked(invoke).mockResolvedValue('C:\\temp\\downloaded.png');
      const { result } = renderHook(() => useAssetFetch(), { wrapper: createWrapper() });

      result.current.mutate('character.png');

      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true);
      });

      expect(mockMutateFn).toHaveBeenCalledWith('fetch_missing_asset', {
        assetName: 'character.png',
      });
    });
  });
});
