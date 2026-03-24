/**
 * US-3.3: CreateObjectModal — form for manually creating a new game object.
 * Uses react-hook-form + zod for validation, reuses patterns from EditObjectModal.
 */

import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { X } from 'lucide-react';
import { useMemo, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { useCreateObject, useGameSchema } from '../../hooks/useObjects';
import { useActiveGame } from '../../hooks/useActiveGame';
import { toast } from '../../stores/useToastStore';
import { type FilterDef, ItemStatus } from '../../types/object';

const createSchema = z.object({
  name: z.string().min(2, 'Name must be at least 2 characters').max(255, 'Name is too long'),
  object_type: z.string().min(1, 'Object type is required'),
  sub_category: z.string().optional(),
  thumbnail_url: z.string().optional(),
  metadata: z.record(z.string(), z.unknown()).optional(),
});

type CreateFormData = z.infer<typeof createSchema>;

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

  const {
    register,
    handleSubmit,
    reset,
    watch,
    setValue,
    formState: { errors },
  } = useForm<CreateFormData>({
    resolver: zodResolver(createSchema),
    defaultValues: {
      name: '',
      object_type: '',
      sub_category: '',
      thumbnail_url: '',
      metadata: {},
    },
  });

  // Track selected category for dynamic metadata fields
  // eslint-disable-next-line react-hooks/incompatible-library
  const objectType = watch('object_type');

  // Derive per-category filters from schema
  const categoryFilters: FilterDef[] = useMemo(() => {
    if (!gameSchema || !objectType) return [];
    const cat = gameSchema.categories.find((c) => c.name === objectType);
    return cat?.filters ?? [];
  }, [gameSchema, objectType]);

  // Reset metadata when category changes (avoid stale keys from previous category)
  const prevCategoryRef = useRef(objectType);
  useEffect(() => {
    if (prevCategoryRef.current !== objectType && objectType) {
      setValue('metadata', {});
    }
    prevCategoryRef.current = objectType;
  }, [objectType, setValue]);

  if (!open || !activeGame) return null;

  const onSubmit = async (data: CreateFormData) => {
    try {
      const newObjectId = await createObject.mutateAsync({
        game_id: activeGame.id,
        name: data.name,
        object_type: data.object_type,
        sub_category: data.sub_category || null,
        metadata: data.metadata || {},
        status: ItemStatus.Enabled,
        folder_path: null,
        thumbnail_url: null,
      });

      // If we have items to import specifically for this new object
      if (pendingPaths && pendingPaths.length > 0 && onImportDropped) {
        onImportDropped(newObjectId.id, data.name, pendingPaths);
      } else {
        toast.success(t('create_modal.success_message', { name: data.name }));
      }

      reset();
      onClose();
    } catch (err) {
      console.error('Failed to create object:', err);
      toast.error(
        t('create_modal.error_message', {
          error: err instanceof Error ? err.message : String(err),
        }),
      );
    }
  };

  const handleClose = () => {
    reset();
    onClose();
  };

  return (
    <div className="modal modal-open">
      <div className="modal-box relative w-11/12 max-w-md">
        <button
          className="btn btn-sm btn-circle absolute right-2 top-2"
          onClick={handleClose}
          aria-label={t('common:actions.close')}
        >
          <X size={16} />
        </button>

        <h3 className="font-bold text-lg mb-4">{t('create_modal.title')}</h3>

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
              {...register('object_type')}
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

          {/* Sub-category (optional, useful for "Other" type) */}
          <div className="form-control w-full">
            <label className="label py-1">
              <span className="label-text">{t('create_modal.sub_category')}</span>
            </label>
            <input
              type="text"
              className="input input-bordered w-full input-sm"
              placeholder={t('create_modal.placeholder_sub')}
              {...register('sub_category')}
            />
          </div>

          {/* Dynamic Metadata Fields — per-category filters */}
          {categoryFilters.length > 0 && (
            <div className="divider text-xs opacity-50 my-1">{t('create_modal.metadata')}</div>
          )}
          {categoryFilters.map((filter) => (
            <div key={filter.key} className="form-control w-full">
              <label className="label py-1">
                <span className="label-text">{filter.label}</span>
              </label>
              {filter.options && filter.options.length > 0 ? (
                <select
                  className="select select-bordered w-full select-sm"
                  {...register(`metadata.${filter.key}`)}
                >
                  <option value="">{t('common:actions.none')}</option>
                  {filter.options.map((opt) => (
                    <option key={opt} value={opt}>
                      {opt}
                    </option>
                  ))}
                </select>
              ) : (
                <input
                  type="text"
                  className="input input-bordered input-sm w-full"
                  {...register(`metadata.${filter.key}`)}
                />
              )}
            </div>
          ))}
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
      <div className="modal-backdrop" onClick={handleClose}></div>
    </div>
  );
}
