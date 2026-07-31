import { describe, expect, it, beforeEach } from 'vitest';
import { QueryClient } from '@tanstack/react-query';
import { applyRuntimeEffects } from './applyOptimisticEffects';
import {
  buildPathInvalidationDescriptor,
  buildPathRewriteDescriptor,
  buildQueryInvalidationDescriptor,
  buildQueryRemovalDescriptor,
} from './descriptorBuilders';
import { mergeRuntimeEffectDescriptors } from './descriptor';
import { useAppStore } from '../../../stores/useAppStore';
import { thumbnailKeys } from '../../../hooks/useThumbnail';
import { detailsKeys } from '../../preview/hooks/usePreviewData';

describe('applyRuntimeEffects', () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient();
    queryClient.clear();
    useAppStore.setState({
      selectedObjectFolderPath: 'ALBEDO',
      explorerSubPath: 'ALBEDO/Variants',
      currentPath: ['ALBEDO', 'Variants'],
      selectedModPath: 'E:/Mods/ALBEDO/Variants/mod.ini',
    });
  });

  it('rewrites runtime selection paths', () => {
    applyRuntimeEffects(
      queryClient,
      buildPathRewriteDescriptor('E:/Mods/ALBEDO/Variants', 'E:/Mods/ALBEDO/Presets', []),
    );

    const state = useAppStore.getState();
    expect(state.explorerSubPath).toBe('ALBEDO/Presets');
    expect(state.currentPath).toEqual(['ALBEDO', 'Presets']);
    expect(state.selectedModPath).toBe('E:/Mods/ALBEDO/Presets/mod.ini');
  });

  it('replaces grid selection using normalized path separators', () => {
    useAppStore.setState({
      gridSelection: new Set(['E:\\Mods\\ALBEDO\\Variant']),
      selectedModPath: 'E:\\Mods\\ALBEDO\\Variant',
    });

    useAppStore.getState().replaceGridSelections([
      { oldPath: 'E:/Mods/ALBEDO/Variant', newPath: 'E:/Mods/ALBEDO/DISABLED Variant' },
    ]);

    const state = useAppStore.getState();
    expect(state.gridSelection.has('E:/Mods/ALBEDO/DISABLED Variant')).toBe(true);
    expect(state.gridSelection.has('E:\\Mods\\ALBEDO\\Variant')).toBe(false);
    expect(state.selectedModPath).toBe('E:/Mods/ALBEDO/DISABLED Variant');
  });

  it('removes thumbnail queries and invalidates detail queries from descriptor effects', () => {
    const thumbnailKey = thumbnailKeys.folder('E:/Mods/ALBEDO');
    const previewKey = detailsKeys.previewImages('E:/Mods/ALBEDO');
    queryClient.setQueryData(thumbnailKey, 'thumb');
    queryClient.setQueryData(previewKey, ['preview.png']);

    applyRuntimeEffects(
      queryClient,
      mergeRuntimeEffectDescriptors(
        buildQueryRemovalDescriptor([thumbnailKey], []),
        buildQueryInvalidationDescriptor([previewKey], []),
      ),
    );

    expect(queryClient.getQueryData(thumbnailKey)).toBeUndefined();
    expect(queryClient.getQueryState(previewKey)?.isInvalidated).toBe(true);
  });

  it('clears stale runtime selection when a path is invalidated', () => {
    applyRuntimeEffects(
      queryClient,
      buildPathInvalidationDescriptor('E:/Mods/ALBEDO/Variants', []),
    );

    const state = useAppStore.getState();
    expect(state.selectedModPath).toBeNull();
    expect(state.explorerSubPath).toBe('ALBEDO/Variants');
    expect(state.currentPath).toEqual(['ALBEDO', 'Variants']);
  });
});
