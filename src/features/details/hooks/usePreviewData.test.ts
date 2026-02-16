import { renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { createWrapper } from '../../../test-utils';
import {
  useModInfo,
  useModIniFiles,
  useModIniDocument,
  useAllModIniDocuments,
  usePreviewImages,
  useSavePreviewImage,
  useRemovePreviewImage,
  useClearPreviewImages,
  useSelectedModPath,
  useWriteModIni,
  useUpdateModInfoDetails,
} from './usePreviewData';
import { useAppStore } from '../../../stores/useAppStore';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('usePreviewData hooks', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.setState({ gridSelection: new Set() });
  });

  it('fetches mod info when folder path is provided', async () => {
    vi.mocked(invoke).mockResolvedValue({ actual_name: 'Mod A' });

    const { result } = renderHook(() => useModInfo('E:/Mods/ModA'), {
      wrapper: createWrapper,
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(invoke).toHaveBeenCalledWith('read_mod_info', { folderPath: 'E:/Mods/ModA' });
  });

  it('fetches ini files list', async () => {
    vi.mocked(invoke).mockResolvedValue([
      { filename: 'config.ini', path: 'E:/Mods/ModA/config.ini' },
    ]);

    const { result } = renderHook(() => useModIniFiles('E:/Mods/ModA'), {
      wrapper: createWrapper,
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(invoke).toHaveBeenCalledWith('list_mod_ini_files', { folderPath: 'E:/Mods/ModA' });
  });

  it('fetches ini document for selected file', async () => {
    vi.mocked(invoke).mockResolvedValue({ mode: 'Structured', raw_lines: [] });

    const { result } = renderHook(() => useModIniDocument('E:/Mods/ModA', 'config.ini'), {
      wrapper: createWrapper,
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(invoke).toHaveBeenCalledWith('read_mod_ini', {
      folderPath: 'E:/Mods/ModA',
      fileName: 'config.ini',
    });
  });

  it('fetches all ini documents for the current folder', async () => {
    vi.mocked(invoke).mockResolvedValue({ mode: 'Structured', raw_lines: [] });

    const files = [
      { filename: 'a.ini', path: 'E:/Mods/ModA/a.ini' },
      { filename: 'b.ini', path: 'E:/Mods/ModA/b.ini' },
    ];

    const { result } = renderHook(() => useAllModIniDocuments('E:/Mods/ModA', files), {
      wrapper: createWrapper,
    });

    await waitFor(() => {
      expect(result.current.every((query) => query.isSuccess)).toBe(true);
    });

    expect(invoke).toHaveBeenCalledWith('read_mod_ini', {
      folderPath: 'E:/Mods/ModA',
      fileName: 'a.ini',
    });
    expect(invoke).toHaveBeenCalledWith('read_mod_ini', {
      folderPath: 'E:/Mods/ModA',
      fileName: 'b.ini',
    });
  });

  it('fetches ordered preview images', async () => {
    vi.mocked(invoke).mockResolvedValue(['E:/Mods/ModA/preview_custom.png']);

    const { result } = renderHook(() => usePreviewImages('E:/Mods/ModA'), {
      wrapper: createWrapper,
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(invoke).toHaveBeenCalledWith('list_mod_preview_images', { folderPath: 'E:/Mods/ModA' });
  });

  it('writes ini line updates with mutation', async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);

    const { result } = renderHook(() => useWriteModIni(), { wrapper: createWrapper });

    await result.current.mutateAsync({
      folderPath: 'E:/Mods/ModA',
      fileName: 'config.ini',
      lineUpdates: [{ line_idx: 1, content: '$swapvar = 1' }],
    });

    expect(invoke).toHaveBeenCalledWith('write_mod_ini', {
      folderPath: 'E:/Mods/ModA',
      fileName: 'config.ini',
      lineUpdates: [{ line_idx: 1, content: '$swapvar = 1' }],
    });
  });

  it('saves preview image with object naming mutation', async () => {
    vi.mocked(invoke).mockResolvedValue('E:/Mods/ModA/preview_keqing.png');

    const { result } = renderHook(() => useSavePreviewImage(), { wrapper: createWrapper });

    await result.current.mutateAsync({
      folderPath: 'E:/Mods/ModA',
      objectName: 'Keqing',
      imageData: [1, 2, 3],
    });

    expect(invoke).toHaveBeenCalledWith('save_mod_preview_image', {
      folderPath: 'E:/Mods/ModA',
      objectName: 'Keqing',
      imageData: [1, 2, 3],
    });
  });

  it('removes one preview image with mutation', async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);

    const { result } = renderHook(() => useRemovePreviewImage(), { wrapper: createWrapper });

    await result.current.mutateAsync({
      folderPath: 'E:/Mods/ModA',
      imagePath: 'E:/Mods/ModA/preview_keqing.png',
    });

    expect(invoke).toHaveBeenCalledWith('remove_mod_preview_image', {
      folderPath: 'E:/Mods/ModA',
      imagePath: 'E:/Mods/ModA/preview_keqing.png',
    });
  });

  it('clears all preview images with mutation', async () => {
    vi.mocked(invoke).mockResolvedValue(['E:/Mods/ModA/preview_keqing.png']);

    const { result } = renderHook(() => useClearPreviewImages(), { wrapper: createWrapper });

    await result.current.mutateAsync({
      folderPath: 'E:/Mods/ModA',
    });

    expect(invoke).toHaveBeenCalledWith('clear_mod_preview_images', {
      folderPath: 'E:/Mods/ModA',
    });
  });

  it('mutation error is exposed to caller', async () => {
    vi.mocked(invoke).mockRejectedValue(new Error('Operation in progress. Please wait.'));

    const { result } = renderHook(() => useWriteModIni(), { wrapper: createWrapper });

    await expect(
      result.current.mutateAsync({
        folderPath: 'E:/Mods/ModA',
        fileName: 'config.ini',
        lineUpdates: [{ line_idx: 1, content: '$swapvar = 1' }],
      }),
    ).rejects.toThrow('Operation in progress. Please wait.');
  });

  it('updates mod info with mutation', async () => {
    vi.mocked(invoke).mockResolvedValue({ actual_name: 'Mod A', description: 'desc' });

    const { result } = renderHook(() => useUpdateModInfoDetails(), { wrapper: createWrapper });

    await result.current.mutateAsync({
      folderPath: 'E:/Mods/ModA',
      update: { actual_name: 'Mod A+', description: 'updated' },
    });

    expect(invoke).toHaveBeenCalledWith('update_mod_info', {
      folderPath: 'E:/Mods/ModA',
      update: { actual_name: 'Mod A+', description: 'updated' },
    });
  });

  it('tracks the latest selected folder path for preview sync', () => {
    useAppStore.setState({
      gridSelection: new Set(['E:/Mods/First', 'E:/Mods/Active']),
    });

    const { result } = renderHook(() => useSelectedModPath(), {
      wrapper: createWrapper,
    });

    expect(result.current).toBe('E:/Mods/Active');
  });
});
