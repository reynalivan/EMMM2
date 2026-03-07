import { UseFormReturn, Controller } from 'react-hook-form';
import type { EditObjectFormData } from './hooks/useEditObjectForm';
import type { GameSchema, FilterDef } from '../../types/object';
import { TagInput } from '../../components/ui/TagInput';

interface EditObjectTabManualProps {
  form: UseFormReturn<EditObjectFormData>;
  gameSchema: GameSchema | null | undefined;
  categoryFilters: FilterDef[];
}

export function EditObjectTabManual({
  form,
  gameSchema,
  categoryFilters,
}: EditObjectTabManualProps) {
  const {
    register,
    control,
    setValue,
    watch,
    formState: { errors },
  } = form;

  const hasCustomSkin = watch('has_custom_skin');

  return (
    <div className="flex min-w-0 flex-1 flex-col gap-3">
      {/* Name */}
      <div className="form-control w-full relative">
        <label className="label py-1">
          <span className="label-text font-medium">Name</span>
        </label>
        <input
          type="text"
          className={`input input-bordered w-full ${errors.name ? 'input-error' : ''}`}
          {...register('name')}
        />
        {errors.name && <span className="text-error text-xs mt-1">{errors.name.message}</span>}
      </div>

      {/* Category Dropdown */}
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
      </div>

      {/* Safe/NSFW Toggle */}
      <div className="form-control mt-2">
        <label className="label cursor-pointer justify-start gap-3 w-fit">
          <input type="checkbox" className="toggle toggle-primary" {...register('is_safe')} />
          <span className="label-text font-medium flex items-center gap-2">Safe Mode (SFW)</span>
        </label>
        <p className="text-[10px] opacity-70 ml-13 -mt-1">
          If unchecked, this item will be hidden when Privacy Mode is enabled.
        </p>
      </div>

      {/* Dynamic Metadata Fields */}
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
              className="input input-bordered input-sm"
              {...register(`metadata.${filter.key}`)}
            />
          )}
        </div>
      ))}

      {/* Tags */}
      <div className="form-control w-full mt-2">
        <label className="label py-1">
          <span className="label-text">Tags</span>
        </label>
        <Controller
          control={control}
          name="tags"
          render={({ field }) => (
            <TagInput
              tags={field.value || []}
              onChange={field.onChange}
              placeholder="Add tags (space/comma to enter)"
            />
          )}
        />
      </div>

      {/* Manual Custom Skin */}
      <div className="form-control w-full mt-4">
        <label className="label py-1">
          <span className="label-text font-bold text-lg">Skin Mapping</span>
        </label>
        <div className="flex gap-4 mt-1 mb-2 px-1">
          <label className="label cursor-pointer justify-start gap-2">
            <input
              type="radio"
              className="radio radio-primary radio-sm"
              value="false"
              checked={!hasCustomSkin}
              onChange={() => {
                setValue('has_custom_skin', false);
                setValue('custom_skin', {
                  name: '',
                  aliases: [],
                  thumbnail_skin_path: '',
                  rarity: '',
                });
              }}
            />
            <span className="label-text">Default / Base Skin</span>
          </label>
          <label className="label cursor-pointer justify-start gap-2">
            <input
              type="radio"
              className="radio radio-primary radio-sm"
              value="true"
              checked={hasCustomSkin}
              onChange={() => setValue('has_custom_skin', true)}
            />
            <span className="label-text">Custom Skin</span>
          </label>
        </div>

        {hasCustomSkin && (
          <div className="flex flex-col gap-3 mt-2 p-4 border border-base-300 rounded-lg bg-base-200/30">
            <div className="form-control w-full">
              <label className="label py-1">
                <span className="label-text">Skin Name</span>
              </label>
              <input
                type="text"
                className={`input input-bordered input-sm w-full ${errors.custom_skin?.name ? 'input-error' : ''}`}
                placeholder="e.g. Red Dead of Night"
                {...register('custom_skin.name')}
              />
              {errors.custom_skin?.name && (
                <label className="label py-0 pt-1">
                  <span className="label-text-alt text-error">
                    {errors.custom_skin.name.message}
                  </span>
                </label>
              )}
            </div>

            <div className="form-control w-full">
              <label className="label py-1">
                <span className="label-text">Thumbnail Path (Optional)</span>
              </label>
              <input
                type="text"
                className="input input-bordered input-sm w-full"
                placeholder="e.g. skins/red_dead_of_night.png"
                {...register('custom_skin.thumbnail_skin_path')}
              />
            </div>

            <div className="form-control w-full">
              <label className="label py-1">
                <span className="label-text">Rarity (Optional)</span>
              </label>
              <input
                type="text"
                className="input input-bordered input-sm w-full"
                placeholder="e.g. 5-Star"
                {...register('custom_skin.rarity')}
              />
            </div>

            <div className="form-control w-full">
              <label className="label py-1">
                <span className="label-text">Aliases</span>
              </label>
              <Controller
                control={control}
                name="custom_skin.aliases"
                render={({ field: { value, onChange } }) => (
                  <TagInput
                    tags={value || []}
                    onChange={onChange}
                    placeholder="Add aliases (space/comma to enter)"
                  />
                )}
              />
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
