import { useTranslation } from 'react-i18next';
import type { FieldValues, Path, UseFormRegister } from 'react-hook-form';
import type { FilterDef } from '../../../types/object';

interface DynamicMetadataFieldsProps<T extends FieldValues> {
  filters: FilterDef[];
  register: UseFormRegister<T>;
}

/**
 * Per-category metadata inputs, driven by the game schema's filter defs.
 * Shared by the create and edit object forms, which render them identically.
 */
export function DynamicMetadataFields<T extends FieldValues>({
  filters,
  register,
}: DynamicMetadataFieldsProps<T>) {
  const { t } = useTranslation(['objects', 'common']);

  return (
    <>
      {filters.map((filter) => (
        <div key={filter.key} className="form-control w-full">
          <label className="label py-1">
            <span className="label-text">{filter.label}</span>
          </label>
          {filter.options && filter.options.length > 0 ? (
            <select
              className="select select-bordered w-full select-sm"
              {...register(`metadata.${filter.key}` as Path<T>)}
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
              {...register(`metadata.${filter.key}` as Path<T>)}
            />
          )}
        </div>
      ))}
    </>
  );
}
