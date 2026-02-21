import { useEffect } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { useQuery } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useUpdateObject } from '../../../hooks/useObjects';
import {
  useRenameMod,
  useUpdateModCategory,
  useUpdateModThumbnail,
  useDeleteModThumbnail,
  useUpdateModInfo,
  ModFolder,
  ModInfo,
} from '../../../hooks/useFolders';
import type { ObjectSummary, GameObject } from '../../../types/object';
import { useActiveGame } from '../../../hooks/useActiveGame';

const schema = z
  .object({
    name: z.string().min(1, 'Name is required'),
    object_type: z.string().min(1, 'Type is required'),
    sub_category: z.string().optional().nullable(),
    is_safe: z.boolean(),
    is_auto_sync: z.boolean(),
    metadata: z.record(z.string(), z.unknown()).optional(),
    tags: z.array(z.string()).optional(),
    has_custom_skin: z.boolean().optional(),
    custom_skin: z
      .object({
        name: z.string().optional(),
        aliases: z.array(z.string()).optional(),
        thumbnail_skin_path: z.string().optional(),
        rarity: z.string().optional(),
      })
      .optional(),
  })
  .superRefine((data, ctx) => {
    if (data.has_custom_skin) {
      if (!data.custom_skin?.name || data.custom_skin.name.trim().length === 0) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'Skin name required',
          path: ['custom_skin', 'name'],
        });
      }
    }
  });

export type EditObjectFormData = z.infer<typeof schema>;

export function useEditObjectForm(
  open: boolean,
  object: ObjectSummary | ModFolder | null,
  onClose: () => void,
  selectedThumbnailPath: string | null,
  thumbnailAction: 'keep' | 'update' | 'delete',
) {
  const { activeGame } = useActiveGame();

  // Mutations
  const updateObject = useUpdateObject();
  const renameMod = useRenameMod();
  const updateCategory = useUpdateModCategory();
  const updateThumbnail = useUpdateModThumbnail();
  const deleteThumbnail = useDeleteModThumbnail();
  const updateInfo = useUpdateModInfo();

  const isPending =
    updateObject.isPending ||
    renameMod.isPending ||
    updateCategory.isPending ||
    updateThumbnail.isPending ||
    deleteThumbnail.isPending ||
    updateInfo.isPending;

  // Detect type
  const isFolder = object && 'path' in object;
  const isObject = object && 'id' in object && !('path' in object);

  // Fetch full details
  const {
    data: fullDetails,
    isLoading: isLoadingDetails,
    isError: isDetailsError,
  } = useQuery({
    queryKey: [
      'edit-details',
      object ? (isFolder ? (object as ModFolder).path : (object as ObjectSummary).id) : 'null',
    ],
    queryFn: async () => {
      if (!object) return null;
      if (isFolder) {
        const folder = object as ModFolder;
        const info = await invoke<ModInfo | null>('read_mod_info', { folderPath: folder.path });
        return { type: 'folder', data: info };
      } else {
        const obj = object as ObjectSummary;
        const data = await invoke<GameObject | null>('get_object', { id: obj.id });
        return { type: 'object', data };
      }
    },
    enabled: !!open && !!object,
    staleTime: 0,
    retry: 1,
  });

  const form = useForm<EditObjectFormData>({
    resolver: zodResolver(schema),
    defaultValues: {
      name: '',
      object_type: '',
      sub_category: '',
      is_safe: true,
      is_auto_sync: false,
      metadata: {},
      tags: [],
      has_custom_skin: false,
      custom_skin: { name: '', aliases: [], thumbnail_skin_path: '', rarity: '' },
    },
  });

  // Reset form when object changes or full details loaded (or query errored)
  useEffect(() => {
    if (!object || isLoadingDetails) return;
    // Wait until query has settled (success or error)
    if (!fullDetails && !isDetailsError) return;

    const defaultName = isFolder ? (object as ModFolder).name : (object as ObjectSummary).name;
    let defaultType = '';
    let defaultSafe: boolean;
    let defaultAutoSync: boolean;
    let defaultMeta: Record<string, unknown> = {};
    let defaultTags: string[] = [];
    let defaultCustomSkin:
      | {
          name: string;
          aliases?: string[];
          thumbnail_skin_path?: string;
          rarity?: string;
        }
      | undefined = undefined;
    let defaultHasCustomSkin: boolean = false;

    if (fullDetails?.type === 'folder' && fullDetails.data) {
      const info = fullDetails.data as ModInfo;
      defaultSafe = info.is_safe;
      defaultAutoSync = info.is_auto_sync;
      if (info.metadata) {
        defaultMeta = info.metadata as Record<string, unknown>;
        if (defaultMeta.custom_skin) {
          defaultCustomSkin = defaultMeta.custom_skin as unknown as NonNullable<
            typeof defaultCustomSkin
          >;
          defaultHasCustomSkin = true;
        }
      }
    } else if (fullDetails?.type === 'object' && fullDetails.data) {
      const obj = fullDetails.data as GameObject;
      defaultType = obj.object_type;
      defaultSafe = obj.is_safe;
      defaultAutoSync = obj.is_auto_sync;
      try {
        if (typeof obj.metadata === 'string') {
          defaultMeta = JSON.parse(obj.metadata);
        } else {
          defaultMeta = obj.metadata as Record<string, unknown>;
        }
        if (defaultMeta.custom_skin) {
          defaultCustomSkin = defaultMeta.custom_skin as unknown as NonNullable<
            typeof defaultCustomSkin
          >;
          defaultHasCustomSkin = true;
        }
      } catch {
        // Ignore JSON parse error
      }
      try {
        if (obj.tags) {
          const parsedTags = typeof obj.tags === 'string' ? JSON.parse(obj.tags) : obj.tags;
          if (Array.isArray(parsedTags)) {
            defaultTags = parsedTags.map(String);
          }
        }
      } catch {
        // Ignore JSON parse error
      }
    } else {
      defaultType = isObject ? (object as ObjectSummary).object_type : '';
      defaultSafe = isObject ? (object as ObjectSummary).is_safe : true;
      defaultAutoSync = isObject ? (object as ObjectSummary).is_auto_sync : false;
      if (isObject && (object as ObjectSummary).tags) {
        try {
          const parsedTags = JSON.parse((object as ObjectSummary).tags);
          if (Array.isArray(parsedTags)) defaultTags = parsedTags.map(String);
        } catch {
          // Ignore JSON parse error
        }
      }
    }

    form.reset({
      name: defaultName,
      object_type: defaultType,
      sub_category: isObject ? (object as ObjectSummary).sub_category : '',
      is_safe: defaultSafe,
      is_auto_sync: defaultAutoSync,
      metadata: defaultMeta,
      tags: defaultTags,
      has_custom_skin: defaultHasCustomSkin,
      custom_skin: defaultCustomSkin || {
        name: '',
        aliases: [],
        thumbnail_skin_path: '',
        rarity: '',
      },
    });
  }, [object, fullDetails, isFolder, isObject, form, isLoadingDetails, isDetailsError]);

  const handleSubmit = form.handleSubmit(async (data) => {
    try {
      const metaStrings: Record<string, string> = {};
      if (data.metadata) {
        Object.entries(data.metadata).forEach(([k, v]) => {
          if (v !== undefined && v !== null) metaStrings[k] = String(v);
        });
      }

      const finalMeta = data.metadata ? { ...data.metadata } : {};
      if (data.has_custom_skin && data.custom_skin?.name) {
        finalMeta.custom_skin = data.custom_skin;
      } else {
        delete finalMeta.custom_skin;
      }

      if (isObject) {
        const obj = object as ObjectSummary;

        await updateObject.mutateAsync({
          id: obj.id,
          updates: {
            name: data.name,
            object_type: data.object_type,
            sub_category: data.sub_category || undefined,
            is_safe: data.is_safe,
            is_auto_sync: data.is_auto_sync,
            metadata: finalMeta,
            tags: data.tags || [],
            thumbnail_path:
              thumbnailAction === 'update'
                ? (selectedThumbnailPath ?? undefined)
                : thumbnailAction === 'delete'
                  ? null
                  : undefined,
          },
        });
      } else if (isFolder) {
        const folder = object as ModFolder;

        // 1. Rename — capture new path to avoid stale reference
        let currentPath = folder.path;
        if (data.name !== folder.name) {
          const result = await renameMod.mutateAsync({
            folderPath: folder.path,
            newName: data.name,
          });
          currentPath = result.new_path;
        }

        // 2. Category
        if (activeGame && data.object_type) {
          await updateCategory.mutateAsync({
            gameId: activeGame.id,
            folderPath: currentPath,
            category: data.object_type,
          });
        }

        // 3. Thumbnail
        if (thumbnailAction === 'update' && selectedThumbnailPath) {
          await updateThumbnail.mutateAsync({
            folderPath: currentPath,
            sourcePath: selectedThumbnailPath,
          });
        } else if (thumbnailAction === 'delete') {
          await deleteThumbnail.mutateAsync(currentPath);
        }

        // 4. Update Info (Safe + Metadata + AutoSync)
        await updateInfo.mutateAsync({
          folderPath: currentPath,
          update: {
            is_safe: data.is_safe,
            is_auto_sync: data.is_auto_sync,
            metadata: metaStrings,
          },
        });
      }
      onClose();
    } catch (e) {
      console.error('Save failed:', e);
      form.setError('root', { message: (e as Error).toString() });
    }
  });

  return {
    form,
    isPending,
    isLoadingDetails,
    handleSubmit,
    isFolder,
    isObject,
  };
}
