import { useTranslation } from 'react-i18next';
import type { SafetyFilter } from '../../stores/appStore/explorerSlice';

interface SafetyFilterControlProps {
  value: SafetyFilter;
  onChange: (value: SafetyFilter) => void;
  compact?: boolean;
}

const FILTERS: SafetyFilter[] = ['all', 'safe', 'unsafe'];

export function SafetyFilterControl({ value, onChange, compact }: SafetyFilterControlProps) {
  const { t } = useTranslation('common');

  if (compact) {
    return (
      <select
        className="select select-xs select-bordered bg-base-200"
        aria-label={t('safety_filter.label', 'Mod safety filter')}
        value={value}
        onChange={(event) => onChange(event.target.value as SafetyFilter)}
      >
        {FILTERS.map((filter) => (
          <option key={filter} value={filter}>
            {t(`safety_filter.${filter}`, filter)}
          </option>
        ))}
      </select>
    );
  }

  return (
    <div className="join" role="group" aria-label={t('safety_filter.label', 'Mod safety filter')}>
      {FILTERS.map((filter) => (
        <button
          key={filter}
          type="button"
          className={`btn btn-xs join-item ${value === filter ? 'btn-primary' : 'btn-ghost'}`}
          aria-pressed={value === filter}
          onClick={() => onChange(filter)}
        >
          {t(`safety_filter.${filter}`, filter)}
        </button>
      ))}
    </div>
  );
}
