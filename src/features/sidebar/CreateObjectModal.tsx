/**
 * US-3.3: CreateObjectModal — form for manually creating a new game object.
 * Uses react-hook-form + zod for validation, reuses patterns from EditObjectModal.
 */

import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { X } from 'lucide-react';
import { useMemo, useEffect, useRef } from 'react';
import { useCreateObject, useGameSchema } from '../../hooks/useObjects';
import { useActiveGame } from '../../hooks/useActiveGame';
import { toast } from '../../stores/useToastStore';
import type { FilterDef } from '../../types/object';

const createSchema = z.object({
  name: z.string().min(2, 'Name must be at least 2 characters').max(255, 'Name is too long'),
  object_type: z.string().min(1, 'Category is required'),
  sub_category: z.string().optional(),
  is_safe: z.boolean(),
  metadata: z.record(z.string(), z.unknown()).optional(),
});

type CreateFormData = z.infer<typeof createSchema>;

interface CreateObjectModalProps {
  open: boolean;
  onClose: () => void;
}

export default function CreateObjectModal({ open, onClose }: CreateObjectModalProps) {
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
      is_safe: true,
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
      await createObject.mutateAsync({
        game_id: activeGame.id,
        name: data.name,
        object_type: data.object_type,
        sub_category: data.sub_category || null,
        is_safe: data.is_safe,
        metadata: data.metadata,
      });
      toast.success(`Object "${data.name}" created`);
      reset();
      onClose();
    } catch (err) {
      console.error('Failed to create object:', err);
      toast.error(`Failed to create object: ${err instanceof Error ? err.message : String(err)}`);
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
          aria-label="Close"
        >
          <X size={16} />
        </button>

        <h3 className="font-bold text-lg mb-4">Create New Object</h3>

        <form onSubmit={handleSubmit(onSubmit)} className="flex flex-col gap-4">
          {/* Name */}
          <div className="form-control w-full">
            <label className="label py-1">
              <span className="label-text font-medium">Name</span>
            </label>
            <input
              type="text"
              className={`input input-bordered w-full ${errors.name ? 'input-error' : ''}`}
              placeholder="e.g. Eula"
              autoFocus
              {...register('name')}
            />
            {errors.name && <span className="text-error text-xs mt-1">{errors.name.message}</span>}
          </div>

          {/* Category */}
          <div className="form-control w-full">
            <label className="label py-1">
              <span className="label-text font-medium">Category</span>
            </label>
            <select
              className={`select select-bordered w-full ${errors.object_type ? 'select-error' : ''}`}
              {...register('object_type')}
            >
              <option value="">Select Category</option>
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
              <span className="label-text">Sub-category (optional)</span>
            </label>
            <input
              type="text"
              className="input input-bordered w-full input-sm"
              placeholder="e.g. Enemy, NPC, VFX"
              {...register('sub_category')}
            />
          </div>

          {/* Dynamic Metadata Fields — per-category filters */}
          {categoryFilters.length > 0 && (
            <div className="divider text-xs opacity-50 my-1">Metadata</div>
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
                  <option value="">None</option>
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

          {/* Safe Mode */}
          <div className="form-control w-full">
            <label className="label cursor-pointer justify-start gap-4 border rounded-lg p-3 hover:bg-base-200 transition-colors">
              <input type="checkbox" className="toggle toggle-success" {...register('is_safe')} />
              <div className="flex flex-col">
                <span className="label-text font-bold">Safe Mode (SFW)</span>
                <span className="label-text-alt opacity-70">Disable to mark as NSFW</span>
              </div>
            </label>
          </div>

          {/* Error feedback */}
          {createObject.isError && (
            <div className="alert alert-error text-sm">
              {createObject.error instanceof Error
                ? createObject.error.message
                : 'Failed to create object'}
            </div>
          )}

          <div className="modal-action border-t border-base-200 pt-4">
            <button
              type="button"
              className="btn"
              onClick={handleClose}
              disabled={createObject.isPending}
            >
              Cancel
            </button>
            <button
              type="submit"
              className="btn btn-primary min-w-[120px]"
              disabled={createObject.isPending}
            >
              {createObject.isPending ? (
                <span className="loading loading-spinner"></span>
              ) : (
                'Create Object'
              )}
            </button>
          </div>
        </form>
      </div>
      <div className="modal-backdrop" onClick={handleClose}></div>
    </div>
  );
}
