import { DynamicMetadataFields } from './DynamicMetadataFields';
import { UseFormReturn, Controller } from 'react-hook-form';
import type { EditObjectFormData } from '../hooks/useEditObjectForm';
import type { GameSchema, FilterDef } from '../../../types/object';
import { TagInput } from '../../../components/ui/TagInput';
import { useTranslation } from 'react-i18next';

interface EditObjectTabManualProps {
  form: UseFormReturn<EditObjectFormData>;
  gameSchema: GameSchema | null | undefined;
  categoryFilters: FilterDef[];
  isObject?: boolean;
}

export function EditObjectTabManual({
  form,
  gameSchema,
  categoryFilters,
}: EditObjectTabManualProps) {
  const { t } = useTranslation(['objects', 'common']);
  const {
    register,
    control,
    setValue,
    watch,
    formState: { errors },
  } = form;
  const hasCustomSkin = watch('has_custom_skin');

  // Enabled/disabled is a folder rename owned by the workspace switch toggle,
  // not an editable DB field — a form control here would be silently reverted
  // by the next disk reconcile.

  return (
    <div className="flex min-w-0 flex-1 flex-col gap-3">
      {/* Name */}
      <div className="form-control w-full relative">
        <label className="label py-1">
          <span className="label-text font-medium">{t('create_modal.name')}</span>
        </label>
        <input
          type="text"
          className={`input input-bordered w-full ${errors.name ? 'input-error' : ''}`}
          placeholder={t('create_modal.placeholder_name')}
          {...register('name')}
        />
        {errors.name && <span className="text-error text-xs mt-1">{errors.name.message}</span>}
      </div>

      {/* Category Dropdown */}
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
      </div>

      {/* Dynamic Metadata Fields */}
      <DynamicMetadataFields filters={categoryFilters} register={register} />

      {/* Tags */}
      <div className="form-control w-full mt-2">
        <label className="label py-1">
          <span className="label-text">{t('edit_modal.aliases')}</span>
        </label>
        <Controller
          control={control}
          name="tags"
          render={({ field }) => (
            <TagInput
              tags={field.value || []}
              onChange={field.onChange}
              placeholder={t('edit_modal.placeholder_tag')}
            />
          )}
        />
      </div>

      {/* Manual Custom Skin */}
      <div className="form-control w-full mt-4">
        <label className="label py-1">
          <span className="label-text font-bold text-lg">{t('edit_modal.skin_mapping')}</span>
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
            <span className="label-text">{t('edit_modal.default_skin')}</span>
          </label>
          <label className="label cursor-pointer justify-start gap-2">
            <input
              type="radio"
              className="radio radio-primary radio-sm"
              value="true"
              checked={hasCustomSkin}
              onChange={() => setValue('has_custom_skin', true)}
            />
            <span className="label-text">{t('edit_modal.custom_skin')}</span>
          </label>
        </div>

        {hasCustomSkin && (
          <div className="flex flex-col gap-3 mt-2 p-4 border border-base-300 rounded-lg bg-base-200/30">
            <div className="form-control w-full">
              <label className="label py-1">
                <span className="label-text">{t('edit_modal.skin_name')}</span>
              </label>
              <input
                type="text"
                className={`input input-bordered input-sm w-full ${errors.custom_skin?.name ? 'input-error' : ''}`}
                placeholder={t('edit_modal.placeholder_skin')}
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
                <span className="label-text">{t('edit_modal.thumbnail_path')}</span>
              </label>
              <input
                type="text"
                className="input input-bordered input-sm w-full"
                placeholder={t('edit_modal.placeholder_thumb')}
                {...register('custom_skin.thumbnail_skin_path')}
              />
            </div>

            <div className="form-control w-full">
              <label className="label py-1">
                <span className="label-text">{t('edit_modal.rarity')}</span>
              </label>
              <input
                type="text"
                className="input input-bordered input-sm w-full"
                placeholder={t('edit_modal.placeholder_rarity')}
                {...register('custom_skin.rarity')}
              />
            </div>

            <div className="form-control w-full">
              <label className="label py-1">
                <span className="label-text">{t('edit_modal.aliases')}</span>
              </label>
              <Controller
                control={control}
                name="custom_skin.aliases"
                render={({ field: { value, onChange } }) => (
                  <TagInput
                    tags={value || []}
                    onChange={onChange}
                    placeholder={t('edit_modal.placeholder_alias')}
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
