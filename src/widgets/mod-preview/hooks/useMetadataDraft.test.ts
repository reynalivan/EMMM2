import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { toast } from '@/shared/ui/toast';
import { useMetadataDraft, type MetadataDraftValues } from './useMetadataDraft';

vi.mock('@/shared/ui/toast', () => ({
  toast: { success: vi.fn(), warning: vi.fn(), error: vi.fn() },
}));

const source: MetadataDraftValues = {
  actual_name: 'Mod A',
  author: 'Original author',
  version: '1.0',
  description: 'Original\nmultiline description',
};

function deferredSave() {
  let resolve!: (values: MetadataDraftValues) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<MetadataDraftValues>((finish, fail) => {
    resolve = finish;
    reject = fail;
  });
  return { promise, resolve, reject };
}

type MetadataContext = Parameters<typeof useMetadataDraft>[0];

function metadataContext(onSave: MetadataContext['onSave']): MetadataContext {
  return {
    activePath: 'E:/Mods/A',
    selectedPath: 'E:/Mods/A',
    gameId: 'game-1',
    filesystemIdentity: 'physical-A',
    fallbackTitle: 'A',
    source,
    onSave,
  };
}

function usePathDraft(
  params: Omit<
    Parameters<typeof useMetadataDraft>[0],
    'gameId' | 'selectedPath' | 'filesystemIdentity'
  >,
) {
  return useMetadataDraft({
    ...params,
    gameId: 'game-1',
    selectedPath: params.activePath,
    filesystemIdentity: null,
  });
}

describe('useMetadataDraft', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
  });

  afterEach(() => vi.useRealTimers());

  it('retains a draft and baseline through a selected-folder query gap and saves at its renamed path', async () => {
    const onSave = vi.fn(async (_path: string, values: MetadataDraftValues) => values);
    const initial = metadataContext(onSave);
    const { result, rerender } = renderHook(
      (context: MetadataContext) => useMetadataDraft(context),
      {
        initialProps: initial,
      },
    );
    act(() => result.current.setDescriptionDraft('QA pending toggle'));
    const oldSave = result.current.saveMetadata;
    rerender({ ...initial, selectedPath: 'E:/Mods/DISABLED A', activePath: null, source: null });

    expect(result.current.descriptionDraft).toBe('QA pending toggle');
    expect(result.current.changedFields).toEqual([
      { label: 'Description', oldValue: source.description, newValue: 'QA pending toggle' },
    ]);
    expect(result.current.metadataDirty).toBe(true);
    await act(async () => oldSave());
    await act(async () => result.current.saveMetadata());
    await act(async () => vi.advanceTimersByTimeAsync(5000));
    expect(onSave).not.toHaveBeenCalled();

    rerender({
      ...initial,
      selectedPath: 'E:/Mods/DISABLED A',
      activePath: 'E:/Mods/DISABLED A',
      source: { ...source, author: 'Refreshed author' },
    });
    expect(result.current.descriptionDraft).toBe('QA pending toggle');
    expect(result.current.authorDraft).toBe('Refreshed author');
    await act(async () => vi.advanceTimersByTimeAsync(2500));
    expect(onSave).toHaveBeenCalledOnce();
    expect(onSave).toHaveBeenCalledWith('E:/Mods/DISABLED A', {
      ...source,
      author: 'Refreshed author',
      description: 'QA pending toggle',
    });
    expect(result.current.metadataDirty).toBe(false);
  });

  it.each(['acknowledgement', 'failure'])(
    'ignores an old-path save %s while preserving the latest edit at the destination',
    async (outcome) => {
      const pending = deferredSave();
      const onSave = vi
        .fn()
        .mockReturnValueOnce(pending.promise)
        .mockImplementationOnce(async (_path: string, values: MetadataDraftValues) => values);
      const initial = metadataContext(onSave);
      const { result, rerender } = renderHook(
        (context: MetadataContext) => useMetadataDraft(context),
        {
          initialProps: initial,
        },
      );
      act(() => result.current.setDescriptionDraft('Old-path save'));
      let save!: Promise<void>;
      act(() => {
        save = result.current.saveMetadata();
      });
      act(() => result.current.setDescriptionDraft('Latest edit'));
      rerender({ ...initial, selectedPath: 'E:/Mods/DISABLED A', activePath: null, source: null });
      rerender({
        ...initial,
        selectedPath: 'E:/Mods/DISABLED A',
        activePath: 'E:/Mods/DISABLED A',
      });
      await act(async () => {
        if (outcome === 'acknowledgement')
          pending.resolve({ ...source, description: 'Old-path save' });
        else pending.reject(new Error('Old path no longer exists'));
        await save;
      });
      expect(toast.success).not.toHaveBeenCalled();
      expect(toast.error).not.toHaveBeenCalled();
      expect(result.current.descriptionDraft).toBe('Latest edit');
      expect(result.current.metadataDirty).toBe(true);
      await act(async () => result.current.saveMetadata());
      expect(onSave).toHaveBeenLastCalledWith('E:/Mods/DISABLED A', {
        ...source,
        description: 'Latest edit',
      });
    },
  );

  it.each(['different physical identity', 'different game', 'explicit deselection'])(
    'resets the draft for %s',
    (change) => {
      const initial = metadataContext(vi.fn());
      const { result, rerender } = renderHook(
        (context: MetadataContext) => useMetadataDraft(context),
        {
          initialProps: initial,
        },
      );
      act(() => result.current.setDescriptionDraft('Local edit'));
      if (change === 'different physical identity') {
        rerender({ ...initial, filesystemIdentity: 'replacement-A' });
      } else if (change === 'different game') {
        rerender({ ...initial, gameId: 'game-2', activePath: null, source: null });
      } else {
        rerender({ ...initial, selectedPath: null, activePath: null, source: null });
      }
      expect(result.current.metadataDirty).toBe(false);
      expect(result.current.descriptionDraft).toBe(
        change === 'different physical identity' ? source.description : '',
      );
    },
  );

  it('does not transfer a draft across rewritten paths without filesystem identity', () => {
    const initial = { ...metadataContext(vi.fn()), filesystemIdentity: null };
    const { result, rerender } = renderHook(
      (context: MetadataContext) => useMetadataDraft(context),
      {
        initialProps: initial,
      },
    );
    act(() => result.current.setDescriptionDraft('Local edit'));
    rerender({ ...initial, selectedPath: 'E:/Mods/DISABLED A', activePath: null, source: null });
    rerender({ ...initial, selectedPath: 'E:/Mods/DISABLED A', activePath: 'E:/Mods/DISABLED A' });
    expect(result.current.descriptionDraft).toBe(source.description);
    expect(result.current.metadataDirty).toBe(false);
  });

  it.each(['', source.description])(
    'keeps a cleared description across a save refresh and acknowledgement (baseline %j)',
    async (baseline) => {
      const pending = deferredSave();
      const onSave = vi
        .fn()
        .mockReturnValueOnce(pending.promise)
        .mockImplementationOnce(async (_path: string, values: MetadataDraftValues) => values);
      const { result, rerender } = renderHook(
        (values: MetadataDraftValues) =>
          usePathDraft({ activePath: 'A', fallbackTitle: 'A', source: values, onSave }),
        { initialProps: { ...source, description: baseline } },
      );
      act(() => result.current.setDescriptionDraft('Saved\nnonempty description'));
      let save!: Promise<void>;
      act(() => {
        save = result.current.saveMetadata();
      });
      act(() => result.current.setDescriptionDraft(''));
      const saved = { ...source, description: 'Saved\nnonempty description' };
      rerender(saved);

      expect(result.current.descriptionDraft).toBe('');
      expect(result.current.metadataDirty).toBe(true);
      await act(async () => {
        pending.resolve(saved);
        await save;
      });
      expect(result.current.descriptionDraft).toBe('');
      expect(result.current.metadataDirty).toBe(true);
      await act(async () => result.current.saveMetadata());

      expect(onSave).toHaveBeenLastCalledWith('A', { ...source, description: '' });
      expect(result.current.metadataDirty).toBe(false);
    },
  );

  it('hydrates clean fields while preserving locally edited fields', () => {
    const { result, rerender } = renderHook(
      (values: MetadataDraftValues) =>
        usePathDraft({ activePath: 'A', fallbackTitle: 'A', source: values, onSave: vi.fn() }),
      { initialProps: source },
    );
    act(() => result.current.setDescriptionDraft('Local description'));
    rerender({ ...source, author: 'Updated author', description: 'External description' });

    expect(result.current.authorDraft).toBe('Updated author');
    expect(result.current.descriptionDraft).toBe('Local description');
    expect(result.current.changedFields).toEqual([
      { label: 'Description', oldValue: 'External description', newValue: 'Local description' },
    ]);
    act(() => result.current.discardMetadata());
    expect(result.current.descriptionDraft).toBe('External description');
    expect(result.current.metadataDirty).toBe(false);
  });

  it('autosaves the latest empty description after an older save finishes', async () => {
    const pending = deferredSave();
    const onSave = vi
      .fn()
      .mockReturnValueOnce(pending.promise)
      .mockImplementationOnce(async (_path: string, values: MetadataDraftValues) => values);
    const { result, rerender } = renderHook(
      (values: MetadataDraftValues) =>
        usePathDraft({ activePath: 'A', fallbackTitle: 'A', source: values, onSave }),
      { initialProps: source },
    );
    act(() => result.current.setDescriptionDraft('Earlier save'));
    await act(async () => vi.advanceTimersByTimeAsync(2500));
    expect(onSave).toHaveBeenCalledOnce();
    act(() => result.current.setDescriptionDraft(''));
    const saved = { ...source, description: 'Earlier save' };
    rerender(saved);
    await act(async () => pending.resolve(saved));
    await act(async () => vi.advanceTimersByTimeAsync(2499));
    expect(onSave).toHaveBeenCalledOnce();
    await act(async () => vi.advanceTimersByTimeAsync(1));

    expect(onSave).toHaveBeenLastCalledWith('A', { ...source, description: '' });
    expect(result.current.descriptionDraft).toBe('');
    expect(result.current.metadataDirty).toBe(false);
  });

  it.each([false, true])(
    'ignores a late acknowledgement after switching selection (return %s)',
    async (returnToA) => {
      const pending = deferredSave();
      const onSave = vi.fn().mockReturnValue(pending.promise);
      const { result, rerender } = renderHook(
        ({ path, values }: { path: string; values: MetadataDraftValues }) =>
          usePathDraft({ activePath: path, fallbackTitle: path, source: values, onSave }),
        { initialProps: { path: 'A', values: source } },
      );
      act(() => result.current.setDescriptionDraft('Old save'));
      let save!: Promise<void>;
      act(() => {
        save = result.current.saveMetadata();
      });
      const next = { ...source, actual_name: 'Mod B', description: 'B description' };
      rerender({ path: 'B', values: next });
      if (returnToA) rerender({ path: 'A', values: source });
      await act(async () => {
        pending.resolve({ ...source, description: 'Old save' });
        await save;
      });

      expect(result.current.descriptionDraft).toBe(
        returnToA ? source.description : next.description,
      );
      expect(result.current.metadataDirty).toBe(false);
      expect(toast.success).not.toHaveBeenCalled();
    },
  );

  it('ignores an older acknowledgement after a newer save succeeded', async () => {
    const older = deferredSave();
    const newer = deferredSave();
    const onSave = vi.fn().mockReturnValueOnce(older.promise).mockReturnValueOnce(newer.promise);
    const { result } = renderHook(() =>
      usePathDraft({ activePath: 'A', fallbackTitle: 'A', source, onSave }),
    );
    act(() => result.current.setDescriptionDraft('Older'));
    let olderSave!: Promise<void>;
    act(() => {
      olderSave = result.current.saveMetadata();
    });
    act(() => result.current.setDescriptionDraft('Newer'));
    let newerSave!: Promise<void>;
    act(() => {
      newerSave = result.current.saveMetadata();
    });
    await act(async () => {
      newer.resolve({ ...source, description: 'Newer' });
      await newerSave;
    });
    await act(async () => {
      older.resolve({ ...source, description: 'Older' });
      await olderSave;
    });

    expect(result.current.descriptionDraft).toBe('Newer');
    expect(result.current.metadataDirty).toBe(false);
  });

  it('retains dirty edits on failure without scheduling repeated retries', async () => {
    const onSave = vi.fn().mockRejectedValue(new Error('Save failed'));
    const { result } = renderHook(() =>
      usePathDraft({ activePath: 'A', fallbackTitle: 'A', source, onSave }),
    );
    act(() => result.current.setDescriptionDraft(''));
    await act(async () => vi.advanceTimersByTimeAsync(2500));
    expect(result.current.descriptionDraft).toBe('');
    expect(result.current.metadataDirty).toBe(true);
    expect(toast.error).toHaveBeenCalledOnce();
    await act(async () => vi.advanceTimersByTimeAsync(10000));
    expect(onSave).toHaveBeenCalledOnce();
  });
});
