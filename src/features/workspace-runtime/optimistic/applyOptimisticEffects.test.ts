import { describe, expect, it, beforeEach } from 'vitest';
import { QueryClient } from '@tanstack/react-query';
import { applyRuntimeEffects } from './applyOptimisticEffects';
import {
  buildObjectCountDeltaDescriptor,
  buildPathInvalidationDescriptor,
  buildPathRewriteDescriptor,
  buildQueryInvalidationDescriptor,
  buildQueryRemovalDescriptor,
} from './descriptorBuilders';
import { mergeRuntimeEffectDescriptors } from './descriptor';
import { objectKeys } from '../../../hooks/objectQueryCache';
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

  it('patches workspace object enabled count deterministically', () => {
    queryClient.setQueryData(
      objectKeys.list({
        game_id: 'genshin',
        safe_mode: false,
        object_type: null,
        search_query: null,
        meta_filters: null,
        sort_by: null,
        status_filter: null,
      }),
      [
        {
          id: 'obj-1',
          name: 'Alpha',
          folder_path: 'Alpha',
          object_type: 'Character',
          sub_category: null,
          status: 'active',
          metadata: '{}',
          tags: '[]',
          hash_db: null,
          custom_skins: null,
          thumbnail_path: null,
          is_auto_sync: false,
          mod_count: 4,
          enabled_count: 1,
          is_pinned: false,
          has_naming_conflict: false,
          matched_alias_name: null,
          is_object_disabled: false,
        },
      ],
    );

    applyRuntimeEffects(queryClient, buildObjectCountDeltaDescriptor('obj-1', 2, []));

    const list = queryClient.getQueryData<
      {
        id: string;
        enabled_count: number;
      }[]
    >(
      objectKeys.list({
        game_id: 'genshin',
        safe_mode: false,
        object_type: null,
        search_query: null,
        meta_filters: null,
        sort_by: null,
        status_filter: null,
      }),
    );

    expect(list?.[0]?.enabled_count).toBe(3);
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
