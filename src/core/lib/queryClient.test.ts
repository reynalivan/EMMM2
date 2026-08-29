import { describe, it, expect } from 'vitest';
import { queryClient } from './queryClient';
import { QueryClient } from '@tanstack/react-query';

describe('queryClient', () => {
  it('should be an instance of QueryClient', () => {
    expect(queryClient).toBeInstanceOf(QueryClient);
  });

  it('should have the correct default options configured', () => {
    const defaultOptions = queryClient.getDefaultOptions();
    expect(defaultOptions.queries?.staleTime).toBe(300000); // 5 minutes
    expect(defaultOptions.queries?.retry).toBe(1);
    expect(defaultOptions.queries?.refetchOnWindowFocus).toBe(false);
  });
});
