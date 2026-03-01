import { vi, describe, it, expect, beforeEach } from 'vitest';
import { useToastStore, toast } from './useToastStore';

describe('useToastStore', () => {
  beforeEach(() => {
    // Clear state before each test
    useToastStore.setState({ toasts: [] });
    vi.restoreAllMocks();
  });

  it('addToast should add a toast to the store with default duration', () => {
    const id = toast.success('Upload complete');
    const state = useToastStore.getState();
    expect(state.toasts.length).toBe(1);
    expect(state.toasts[0].id).toBe(id);
    expect(state.toasts[0].message).toBe('Upload complete');
    expect(state.toasts[0].type).toBe('success');
    expect(state.toasts[0].duration).toBe(3000); // default
  });

  it('removeToast should remove a toast by id', () => {
    const id = toast.info('Updating...');
    expect(useToastStore.getState().toasts.length).toBe(1);

    useToastStore.getState().removeToast(id);
    expect(useToastStore.getState().toasts.length).toBe(0);
  });

  it('should cap the visible toasts at 5 (TC-36-004)', () => {
    const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

    // Try adding 15 toasts rapidly
    for (let i = 0; i < 15; i++) {
      toast.error(`Error ${i}`);
    }

    const state = useToastStore.getState();
    // Only 5 should be accepted
    expect(state.toasts.length).toBe(5);

    // Check that warning was called 10 times
    expect(consoleSpy).toHaveBeenCalledTimes(10);
    expect(consoleSpy).toHaveBeenCalledWith(
      'Toast cap reached. Dropped toast:',
      'error',
      expect.any(String),
    );
  });
});
