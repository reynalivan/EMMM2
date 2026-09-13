/**
 * US-3.3: CreateObjectModal — form for manually creating a new game object.
 * Uses react-hook-form + zod for validation, reuses patterns from EditObjectModal.
 */

import { DynamicMetadataFields } from './DynamicMetadataFields';
import { formatAppError } from '../../../shared/lib/appError';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { ImageIcon, X } from 'lucide-react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { convertFileSrc } from '@tauri-apps/api/core';
import { useMemo, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useGameSchema } from '../hooks/useObjectQueries';
import { useCreateObject } from '@/features/workspace-runtime';
import { useDialogSync } from '@/shared/lib/hooks/useDialogSync';
import type { JsonValue } from '@/entities/game-object';
import { useActiveGame } from '@/entities/game';
import { toast } from '@/shared/ui/toast';
import { type FilterDef, ItemStatus } from '@/entities/game-object';

type CreateFormData = {
  name: string;
  object_type: string;
  sub_category?: string;
  metadata?: Record<string, unknown>;
};

function createSchema(t: (key: string) => string) {
  return z.object({
    name: z
      .string()
      .min(2, t('create_modal.validation.name_too_short'))
      .max(255, t('create_modal.validation.name_too_long')),
    object_type: z.string().min(1, t('create_modal.validation.object_type_required')),
    sub_category: z.string().optional(),
    metadata: z.record(z.string(), z.unknown()).optional(),
  });
}

type PendingThumbnail =
  | { kind: 'file'; source_path: string; previewSrc: string }
  | { kind: 'clipboard'; image_data: number[]; previewSrc: string }
  | { kind: 'url'; url: string; previewSrc: string };

interface CreateObjectModalProps {
  open: boolean;
  pendingPaths?: string[] | null;
  onImportDropped?: (newObjectId: string, objectName: string, paths: string[]) => void;
  onClose: () => void;
}

export default function CreateObjectModal({
  open,
  pendingPaths,
  onImportDropped,
  onClose,
}: CreateObjectModalProps) {
  const { t } = useTranslation(['objects', 'common']);
  const { activeGame } = useActiveGame();
  const { data: gameSchema } = useGameSchema();
  const createObject = useCreateObject();
  const [thumbnail, setThumbnail] = useState<PendingThumbnail | null>(null);
  const [thumbnailUrl, setThumbnailUrl] = useState('');
  const dialogRef = useRef<HTMLDialogElement>(null);
  const validationSchema = useMemo(() => createSchema(t), [t]);

  const {
    register,
    handleSubmit,
    reset,
    watch,
    setValue,
    formState: { errors },
  } = useForm<CreateFormData>({
    resolver: zodResolver(validationSchema),
    defaultValues: {
      name: '',
      object_type: '',
      sub_category: '',
      metadata: {},
    },
  });

  // Track selected category for dynamic metadata fields
  // eslint-disable-next-line react-hooks/incompatible-library
  const objectType = watch('object_type');
  const objectTypeField = register('object_type');

  // Derive per-category filters from schema
  const categoryFilters: FilterDef[] = useMemo(() => {
    if (!gameSchema || !objectType) return [];
    const cat = gameSchema.categories.find((c) => c.name === objectType);
    return cat?.filters ?? [];
  }, [gameSchema, objectType]);

  const categorySubcategories = useMemo(() => {
    if (!gameSchema || !objectType) return [];
    return (
      gameSchema.categories.find((category) => category.name === objectType)?.subcategories ?? []
    );
  }, [gameSchema, objectType]);

  // Reset metadata when category changes (avoid stale keys from previous category)
  const prevCategoryRef = useRef(objectType);
  useEffect(() => {
    if (prevCategoryRef.current !== objectType && objectType) {
      setValue('metadata', {});
    }
    prevCategoryRef.current = objectType;
  }, [objectType, setValue]);

  useEffect(
    () => () => {
      if (thumbnail?.kind === 'clipboard') {
        URL.revokeObjectURL(thumbnail.previewSrc);
      }
    },
    [thumbnail],
  );

  const replaceThumbnail = (nextThumbnail: PendingThumbnail | null) => {
    setThumbnail(nextThumbnail);
  };

  const selectThumbnailFile = async () => {
    try {
      const selected = await openDialog({
        multiple: false,
        filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
      });
      if (selected && typeof selected === 'string') {
        replaceThumbnail({
          kind: 'file',
          source_path: selected,
          previewSrc: convertFileSrc(selected),
        });
      }
    } catch (error) {
      toast.error(t('create_modal.thumbnail.file_error', { error: formatAppError(error) }));
    }
  };

  const pasteThumbnail = async () => {
    try {
      const clipboardItems = await navigator.clipboard.read();
      for (const item of clipboardItems) {
        const imageType = item.types.find((type) => type.startsWith('image/'));
        if (!imageType) continue;
        const image = await item.getType(imageType);
        replaceThumbnail({
          kind: 'clipboard',
          image_data: Array.from(new Uint8Array(await image.arrayBuffer())),
          previewSrc: URL.createObjectURL(image),
        });
        return;
      }
      toast.warning(t('create_modal.thumbnail.no_clipboard_image'));
    } catch (error) {
      toast.error(t('create_modal.thumbnail.paste_error', { error: formatAppError(error) }));
    }
  };

  const useThumbnailUrl = () => {
    const url = thumbnailUrl.trim();
    if (!url) return;
    replaceThumbnail({ kind: 'url', url, previewSrc: url });
    setThumbnailUrl('');
  };

  useDialogSync(dialogRef, open && Boolean(activeGame));

  if (!open || !activeGame) return null;

  const onSubmit = async (data: CreateFormData) => {
    try {
      const newObjectId = await createObject.mutateAsync({
        game_id: activeGame.id,
        name: data.name,
        object_type: data.object_type,
        sub_category: data.sub_category || null,
        metadata: (data.metadata || {}) as JsonValue,
        status: ItemStatus.Enabled,
        folder_path: null,
        thumbnail: thumbnail
          ? thumbnail.kind === 'file'
            ? { kind: 'file', source_path: thumbnail.source_path }
            : thumbnail.kind === 'clipboard'
              ? { kind: 'clipboard', image_data: thumbnail.image_data }
              : { kind: 'url', url: thumbnail.url }
          : null,
        thumbnail_url: null,
        hash_db: null,
        custom_skins: null,
      });

      // If we have items to import specifically for this new object
      if (pendingPaths && pendingPaths.length > 0 && onImportDropped) {
        onImportDropped(newObjectId.id, data.name, pendingPaths);
      } else {
        toast.success(t('create_modal.success_message', { name: data.name }));
      }

      reset();
      replaceThumbnail(null);
      setThumbnailUrl('');
      onClose();
    } catch (err) {
      console.error('Failed to create object:', err);
      toast.error(
        t('create_modal.error_message', {
          error: formatAppError(err),
        }),
      );
    }
  };

  const handleClose = () => {
    reset();
    replaceThumbnail(null);
    setThumbnailUrl('');
    onClose();
  };

  return (
    <dialog
      ref={dialogRef}
      className="modal modal-bottom sm:modal-middle"
      aria-labelledby="create-object-title"
      onClose={handleClose}
    >
      <div className="modal-box relative w-11/12 max-w-md">
        <button
          className="btn btn-sm btn-circle absolute right-2 top-2"
          onClick={handleClose}
          aria-label={t('common:actions.close')}
        >
          <X size={16} />
        </button>

        <h3 id="create-object-title" className="font-bold text-lg mb-4">
          {t('create_modal.title')}
        </h3>

        <form onSubmit={handleSubmit(onSubmit)} className="flex flex-col gap-4">
          {/* Name */}
          <div className="form-control w-full">
            <label className="label py-1">
              <span className="label-text font-medium">{t('create_modal.name')}</span>
            </label>
            <input
              type="text"
              className={`input input-bordered w-full ${errors.name ? 'input-error' : ''}`}
              placeholder={t('create_modal.placeholder_name')}
              autoFocus
              {...register('name')}
            />
            {errors.name && <span className="text-error text-xs mt-1">{errors.name.message}</span>}
          </div>

          {/* Category */}
          <div className="form-control w-full">
            <label className="label py-1">
              <span className="label-text font-medium">{t('create_modal.category')}</span>
            </label>
            <select
              className={`select select-bordered w-full ${errors.object_type ? 'select-error' : ''}`}
              {...objectTypeField}
              onChange={(event) => {
                objectTypeField.onChange(event);
                setValue('sub_category', '');
              }}
            >
              <option value="">{t('create_modal.select_category')}</option>
              {gameSchema?.categories.map((cat) => (
                <option key={cat.name} value={cat.name}>
                  {cat.label ?? cat.name}
                </option>
              ))}
            </select>
            {errors.object_type && (
              <span className="text-error text-xs mt-1">{errors.object_type.message}</span>
            )}
          </div>

          {categorySubcategories.length > 0 && (
            <div className="form-control w-full">
              <label className="label py-1">
                <span className="label-text">{t('create_modal.sub_category')}</span>
              </label>
              <select
                className="select select-bordered w-full input-sm"
                {...register('sub_category')}
              >
                <option value="">{t('common:actions.none')}</option>
                {categorySubcategories.map((subcategory) => (
                  <option key={subcategory} value={subcategory}>
                    {subcategory}
                  </option>
                ))}
              </select>
            </div>
          )}

          <div className="form-control w-full gap-2">
            <label className="label py-1">
              <span className="label-text font-medium">{t('create_modal.thumbnail.label')}</span>
            </label>
            <div className="flex items-center gap-3 rounded-box border border-base-300 bg-base-200/30 p-3">
              <div className="flex h-16 w-16 shrink-0 items-center justify-center overflow-hidden rounded-box bg-base-300">
                {thumbnail ? (
                  <img
                    src={thumbnail.previewSrc}
                    alt={t('create_modal.thumbnail.alt')}
                    className="h-full w-full object-cover"
                  />
                ) : (
                  <ImageIcon className="opacity-40" size={28} aria-hidden="true" />
                )}
              </div>
              <div className="flex min-w-0 flex-1 flex-wrap gap-2">
                <button
                  type="button"
                  className="btn btn-sm btn-outline"
                  onClick={selectThumbnailFile}
                >
                  {t('create_modal.thumbnail.select_file')}
                </button>
                <button type="button" className="btn btn-sm btn-outline" onClick={pasteThumbnail}>
                  {t('create_modal.thumbnail.paste')}
                </button>
                {thumbnail && (
                  <button
                    type="button"
                    className="btn btn-sm btn-ghost btn-square"
                    onClick={() => replaceThumbnail(null)}
                    aria-label={t('create_modal.thumbnail.remove')}
                    title={t('create_modal.thumbnail.remove')}
                  >
                    <X size={16} aria-hidden="true" />
                  </button>
                )}
              </div>
            </div>
            <div className="join w-full">
              <input
                type="url"
                className="input input-bordered input-sm join-item min-w-0 flex-1"
                value={thumbnailUrl}
                placeholder={t('create_modal.thumbnail.url_placeholder')}
                aria-label={t('create_modal.thumbnail.url_placeholder')}
                onChange={(event) => setThumbnailUrl(event.target.value)}
              />
              <button
                type="button"
                className="btn btn-sm btn-outline join-item"
                onClick={useThumbnailUrl}
                disabled={!thumbnailUrl.trim()}
              >
                {t('create_modal.thumbnail.use_url')}
              </button>
            </div>
          </div>

          {/* Dynamic Metadata Fields — per-category filters */}
          {categoryFilters.length > 0 && (
            <div className="divider text-xs text-muted my-1">{t('create_modal.metadata')}</div>
          )}
          <DynamicMetadataFields filters={categoryFilters} register={register} />
          {/* Error feedback */}
          {createObject.isError && (
            <div className="alert alert-error text-sm">
              {createObject.error instanceof Error
                ? createObject.error.message
                : t('create_modal.error_generic')}
            </div>
          )}

          <div className="modal-action border-t border-base-200 pt-4">
            <button
              type="button"
              className="btn"
              onClick={handleClose}
              disabled={createObject.isPending}
            >
              {t('common:actions.cancel')}
            </button>
            <button
              type="submit"
              className="btn btn-primary min-w-30"
              disabled={createObject.isPending}
            >
              {createObject.isPending ? (
                <span className="loading loading-spinner"></span>
              ) : (
                t('create_modal.submit')
              )}
            </button>
          </div>
        </form>
      </div>
      <form method="dialog" className="modal-backdrop">
        <button onClick={handleClose}>{t('common:actions.close')}</button>
      </form>
    </dialog>
  );
}
