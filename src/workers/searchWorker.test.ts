import { describe, it, expect, vi, beforeEach } from 'vitest';

describe('searchWorker', () => {
  let onmessageHandler: (e: { data: unknown }) => void;
  let postMessageSpy: ReturnType<typeof vi.fn>;

  beforeEach(async () => {
    postMessageSpy = vi.fn();

    Object.defineProperty(globalThis, 'self', {
      value: {
        postMessage: postMessageSpy,
        set onmessage(handler: (e: { data: unknown }) => void) {
          onmessageHandler = handler;
        },
      },
      writable: true,
      configurable: true,
    });

    vi.resetModules();
    // Dynamic import to execute the script in the mocked self environment
    await import('./searchWorker');
  });

  it('should return null ids if query is empty', () => {
    onmessageHandler({
      data: {
        objects: [{ id: '1', name: 'Alhaitham' }],
        query: '   ',
      },
    });

    expect(postMessageSpy).toHaveBeenCalledWith({ ids: null });
  });

  it('should return matched ids for single token', () => {
    const objects = [
      { id: '1', name: 'Alhaitham' },
      { id: '2', name: 'Amber' },
      { id: '3', name: 'Albedo' },
    ];

    onmessageHandler({
      data: {
        objects,
        query: 'alb',
      },
    });

    expect(postMessageSpy).toHaveBeenCalledWith({ ids: ['3'] });
  });

  it('should handle multi-token substring matching case-insensitively', () => {
    const objects = [
      { id: '1', name: 'Genshin Impact Alhaitham Mod' },
      { id: '2', name: 'Alhaitham Default Outfit' },
      { id: '3', name: 'Amber Outfit' },
    ];

    onmessageHandler({
      data: {
        objects,
        query: 'Alhaitham mod',
      },
    });

    expect(postMessageSpy).toHaveBeenCalledWith({ ids: ['1'] });
  });

  it('should return empty array if no match', () => {
    const objects = [
      { id: '1', name: 'Alhaitham' },
      { id: '2', name: 'Amber' },
    ];

    onmessageHandler({
      data: {
        objects,
        query: 'Zhongli',
      },
    });

    expect(postMessageSpy).toHaveBeenCalledWith({ ids: [] });
  });
});
