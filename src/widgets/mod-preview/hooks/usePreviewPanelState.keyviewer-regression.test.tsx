import { beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, renderHook, waitFor } from '../../../tests/testing/test-utils';
import { usePreviewPanelState } from './usePreviewPanelState';

const fixture = vi.hoisted(() => ({
  runtime: {} as Record<string, unknown>,
  mutate: vi.fn(),
  state: {
    dialogState: { kind: 'closed' },
    previewTransition: { kind: 'idle' },
    previewDirty: false,
  },
}));

vi.mock('./usePreviewRuntime', () => ({ usePreviewRuntime: () => fixture.runtime }));
vi.mock('./useMetadataDraft', () => ({ useMetadataDraft: () => ({ metadataDirty: false }) }));
vi.mock('@/features/workspace-runtime', () => ({
  useWorkspaceRuntimeSelector: (selector: (state: typeof fixture.state) => unknown) =>
    selector(fixture.state),
  dispatchWorkspaceRuntimeEvent: vi.fn(),
}));
vi.mock('@/shared/ui/toast', () => ({ toast: { error: vi.fn(), success: vi.fn() } }));

function documents(key: string, hash: string, lineIdx = 1) {
  const rawLines = ['[KeyDress]'];
  while (rawLines.length < lineIdx) rawLines.push('');
  rawLines.push(`key = ${key}`);
  return [
    {
      fileName: 'mod.ini',
      document: {
        source_hash: hash,
        mode: 'Structured',
        raw_lines: rawLines,
        variables: [],
        key_bindings: [
          { section_name: 'KeyDress', key, back: null, key_line_idx: lineIdx, back_line_idx: null },
        ],
      },
    },
  ];
}

describe('usePreviewPanelState KeyViewer regressions', () => {
  beforeEach(() => {
    cleanup();
    fixture.mutate.mockReset().mockResolvedValue(undefined);
    fixture.runtime = {
      activePath: 'E:/Fixture/Mod',
      activeGameId: 'fixture',
      selectedPath: 'E:/Fixture/Mod',
      iniDocuments: documents('A', 'old-hash'),
      images: [],
      writeModIni: { mutateAsync: fixture.mutate, isPending: false },
    };
  });

  it('uses the original field location and source hash while a stale draft is dirty', async () => {
    const { result, rerender } = renderHook(() => usePreviewPanelState());
    const id = 'mod.ini:KeyDress:key:1';
    await waitFor(() => expect(result.current.draftByField[id]).toBe('A'));
    act(() => result.current.updateEditorField(id, 'B'));
    fixture.runtime = { ...fixture.runtime, iniDocuments: documents('C', 'external-edit-hash', 4) };
    rerender();

    expect(result.current.keyBindSections[0].sections[0].fields[0].id).toBe(id);

    await act(async () => {
      await result.current.saveEditor();
    });

    expect(fixture.mutate).toHaveBeenCalledWith(
      expect.objectContaining({
        expectedSourceHash: 'old-hash',
        lineUpdates: [{ line_idx: 1, content: 'key = B' }],
      }),
    );
  });

  it('preserves edits typed after the submitted snapshot while save is pending', async () => {
    let finishSave: (() => void) | undefined;
    fixture.mutate.mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          finishSave = resolve;
        }),
    );
    const { result, rerender } = renderHook(() => usePreviewPanelState());
    const id = 'mod.ini:KeyDress:key:1';
    await waitFor(() => expect(result.current.draftByField[id]).toBe('A'));
    act(() => result.current.updateEditorField(id, 'B'));
    let pending: Promise<boolean> | undefined;
    act(() => {
      pending = result.current.saveEditor();
    });
    act(() => result.current.updateEditorField(id, 'C'));
    expect(result.current.draftByField[id]).toBe('C');
    fixture.runtime = { ...fixture.runtime, iniDocuments: documents('B', 'saved-hash') };
    rerender();
    await act(async () => {
      finishSave?.();
      expect(await pending).toBe(false);
    });

    expect(result.current.draftByField[id]).toBe('C');
    expect(result.current.hasUnsavedEditorChanges).toBe(true);
    act(() => result.current.updateEditorField(id, 'D'));
    await act(async () => {
      await result.current.saveEditor();
    });
    expect(fixture.mutate).toHaveBeenLastCalledWith(
      expect.objectContaining({ expectedSourceHash: 'saved-hash' }),
    );
  });

  it('keeps an externally changed unsaved file on its original baseline after a partial save', async () => {
    const alpha = { ...documents('A', 'alpha-old')[0], fileName: 'alpha.ini' };
    const beta = { ...documents('A', 'beta-old')[0], fileName: 'beta.ini' };
    fixture.runtime = {
      ...fixture.runtime,
      iniDocuments: [alpha, { ...beta, fileName: 'beta.ini' }],
    };
    fixture.mutate.mockImplementation(({ fileName }: { fileName: string }) =>
      fileName === 'alpha.ini' ? Promise.resolve() : Promise.reject(new Error('stale beta')),
    );
    const { result, rerender } = renderHook(() => usePreviewPanelState());
    const alphaId = 'alpha.ini:KeyDress:key:1';
    await waitFor(() => expect(result.current.draftByField[alphaId]).toBe('A'));
    act(() => {
      result.current.updateEditorField(alphaId, 'B');
      result.current.updateEditorField('beta.ini:KeyDress:key:1', 'C');
    });
    fixture.runtime = {
      ...fixture.runtime,
      iniDocuments: [
        { ...documents('B', 'alpha-new')[0], fileName: 'alpha.ini' },
        { ...documents('EXTERNAL', 'beta-external')[0], fileName: 'beta.ini' },
      ],
    };
    rerender();

    await act(async () => {
      await result.current.saveEditor();
    });

    expect(fixture.mutate).toHaveBeenCalledWith(
      expect.objectContaining({ fileName: 'alpha.ini', expectedSourceHash: 'alpha-old' }),
    );
    expect(fixture.mutate).toHaveBeenCalledWith(
      expect.objectContaining({ fileName: 'beta.ini', expectedSourceHash: 'beta-old' }),
    );
  });

  it('retires pending save confirmation when the user explicitly discards and rebases', async () => {
    const { result, rerender } = renderHook(() => usePreviewPanelState());
    const id = 'mod.ini:KeyDress:key:1';
    await waitFor(() => expect(result.current.draftByField[id]).toBe('A'));
    act(() => result.current.updateEditorField(id, 'B'));
    await act(async () => {
      await result.current.saveEditor();
    });
    fixture.runtime = { ...fixture.runtime, iniDocuments: documents('C', 'discard-baseline') };
    rerender();
    act(() => result.current.discardEditor());
    act(() => result.current.updateEditorField(id, 'D'));
    fixture.runtime = { ...fixture.runtime, iniDocuments: documents('B', 'later-external-hash') };
    rerender();
    await act(async () => {
      await result.current.saveEditor();
    });
    expect(fixture.mutate).toHaveBeenLastCalledWith(
      expect.objectContaining({ expectedSourceHash: 'discard-baseline' }),
    );
  });

  it('does not settle a previous save after navigating away and back to the same path', async () => {
    let finishSave: (() => void) | undefined;
    fixture.mutate.mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          finishSave = resolve;
        }),
    );
    const { result, rerender } = renderHook(() => usePreviewPanelState());
    const id = 'mod.ini:KeyDress:key:1';
    await waitFor(() => expect(result.current.draftByField[id]).toBe('A'));
    act(() => result.current.updateEditorField(id, 'B'));
    let pending: Promise<boolean> | undefined;
    act(() => {
      pending = result.current.saveEditor();
    });
    fixture.runtime = {
      ...fixture.runtime,
      activePath: 'E:/Fixture/Other',
      iniDocuments: documents('X', 'other-hash'),
    };
    rerender();
    fixture.runtime = {
      ...fixture.runtime,
      activePath: 'E:/Fixture/Mod',
      iniDocuments: documents('A', 'return-hash'),
    };
    rerender();
    act(() => result.current.updateEditorField(id, 'D'));
    let saved: boolean | undefined;
    await act(async () => {
      finishSave?.();
      saved = await pending;
    });
    expect(saved).toBe(false);
    expect(result.current.draftByField[id]).toBe('D');
    expect(result.current.hasUnsavedEditorChanges).toBe(true);
  });

  it('keeps a concurrently confirmed file baseline while another file retry settles', async () => {
    fixture.runtime = {
      ...fixture.runtime,
      iniDocuments: [
        { ...documents('A', 'alpha-old')[0], fileName: 'alpha.ini' },
        { ...documents('A', 'beta-old')[0], fileName: 'beta.ini' },
      ],
    };
    let betaAttempts = 0;
    let finishBeta: (() => void) | undefined;
    fixture.mutate.mockImplementation(({ fileName }: { fileName: string }) => {
      if (fileName !== 'beta.ini') return Promise.resolve();
      if (++betaAttempts === 1) return Promise.reject(new Error('retry beta'));
      return new Promise<void>((resolve) => {
        finishBeta = resolve;
      });
    });
    const { result, rerender } = renderHook(() => usePreviewPanelState());
    const alphaId = 'alpha.ini:KeyDress:key:1';
    await waitFor(() => expect(result.current.draftByField[alphaId]).toBe('A'));
    act(() => {
      result.current.updateEditorField(alphaId, 'B');
      result.current.updateEditorField('beta.ini:KeyDress:key:1', 'B');
    });
    await act(async () => {
      expect(await result.current.saveEditor()).toBe(false);
    });
    let retry: Promise<boolean> | undefined;
    act(() => {
      retry = result.current.saveEditor();
    });
    fixture.runtime = {
      ...fixture.runtime,
      iniDocuments: [
        { ...documents('B', 'alpha-confirmed')[0], fileName: 'alpha.ini' },
        { ...documents('A', 'beta-old')[0], fileName: 'beta.ini' },
      ],
    };
    rerender();
    await act(async () => {
      finishBeta?.();
      await retry;
    });
    act(() => result.current.updateEditorField(alphaId, 'C'));
    await act(async () => {
      await result.current.saveEditor();
    });
    expect(fixture.mutate).toHaveBeenLastCalledWith(
      expect.objectContaining({ fileName: 'alpha.ini', expectedSourceHash: 'alpha-confirmed' }),
    );
  });

  it('accepts a clean post-mutation fetch even if an external edit superseded the saved bytes', async () => {
    const { result, rerender } = renderHook(() => usePreviewPanelState());
    const id = 'mod.ini:KeyDress:key:1';
    await waitFor(() => expect(result.current.draftByField[id]).toBe('A'));
    act(() => result.current.updateEditorField(id, 'B'));
    await act(async () => {
      await result.current.saveEditor();
    });
    expect(result.current.hasUnsavedEditorChanges).toBe(false);
    fixture.runtime = {
      ...fixture.runtime,
      iniDocumentsUpdatedAt: 2,
      iniDocuments: documents('C', 'post-save-external'),
    };
    rerender();
    await waitFor(() => expect(result.current.draftByField[id]).toBe('C'));
    act(() => result.current.updateEditorField(id, 'D'));
    await act(async () => {
      await result.current.saveEditor();
    });
    expect(fixture.mutate).toHaveBeenLastCalledWith(
      expect.objectContaining({ expectedSourceHash: 'post-save-external' }),
    );
  });

  it('retains validation errors and keeps editing when a newer invalid draft was typed during save', async () => {
    let finishSave: (() => void) | undefined;
    fixture.mutate.mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          finishSave = resolve;
        }),
    );
    const { result } = renderHook(() => usePreviewPanelState());
    const id = 'mod.ini:KeyDress:key:1';
    await waitFor(() => expect(result.current.draftByField[id]).toBe('A'));
    act(() => result.current.updateEditorField(id, 'B'));
    let pending: Promise<boolean> | undefined;
    act(() => {
      pending = result.current.saveEditor();
    });
    act(() => result.current.updateEditorField(id, 'CTRL+'));
    expect(result.current.fieldErrors[id]).toBeTruthy();
    await act(async () => {
      finishSave?.();
      expect(await pending).toBe(false);
    });
    expect(result.current.fieldErrors[id]).toBeTruthy();
    expect(result.current.hasUnsavedEditorChanges).toBe(true);
  });

  it('retires a superseded save when its newer fetch finished before the other write settled', async () => {
    fixture.runtime = {
      ...fixture.runtime,
      iniDocuments: [
        { ...documents('A', 'alpha-old')[0], fileName: 'alpha.ini' },
        { ...documents('A', 'beta-old')[0], fileName: 'beta.ini' },
      ],
    };
    let finishBeta: (() => void) | undefined;
    fixture.mutate.mockImplementation(({ fileName }: { fileName: string }) =>
      fileName === 'alpha.ini'
        ? Promise.resolve()
        : new Promise<void>((resolve) => {
            finishBeta = resolve;
          }),
    );
    const { result, rerender } = renderHook(() => usePreviewPanelState());
    const alphaId = 'alpha.ini:KeyDress:key:1';
    await waitFor(() => expect(result.current.draftByField[alphaId]).toBe('A'));
    act(() => {
      result.current.updateEditorField(alphaId, 'B');
      result.current.updateEditorField('beta.ini:KeyDress:key:1', 'B');
    });
    let pending: Promise<boolean> | undefined;
    await act(async () => {
      pending = result.current.saveEditor();
      await Promise.resolve();
    });
    fixture.runtime = {
      ...fixture.runtime,
      iniDocumentsUpdatedAt: 2,
      iniDocuments: [
        { ...documents('C', 'alpha-external')[0], fileName: 'alpha.ini' },
        { ...documents('B', 'beta-new')[0], fileName: 'beta.ini' },
      ],
    };
    rerender();
    await act(async () => {
      finishBeta?.();
      await pending;
    });
    expect(result.current.hasUnsavedEditorChanges).toBe(false);
    expect(result.current.draftByField[alphaId]).toBe('C');
  });
});
