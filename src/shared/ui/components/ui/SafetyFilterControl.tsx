import { ChevronDown, Shield, ShieldAlert, ShieldCheck } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { LiquidSurface } from '@/shared/ui/liquid';
export type SafetyFilter = 'all' | 'safe' | 'unsafe';

interface SafetyFilterControlProps {
  value: SafetyFilter;
  onChange: (value: SafetyFilter) => void;
  compact?: boolean;
}

const FILTERS = [
  { value: 'all', icon: Shield, className: 'text-base-content/55' },
  { value: 'safe', icon: ShieldCheck, className: 'text-success' },
  { value: 'unsafe', icon: ShieldAlert, className: 'text-warning' },
] as const;

export function SafetyFilterControl({
  value,
  onChange,
  compact = false,
}: SafetyFilterControlProps) {
  const { t } = useTranslation('common');
  const activeFilter = FILTERS.find((filter) => filter.value === value) ?? FILTERS[0];
  const ActiveIcon = activeFilter.icon;

  const selectFilter = (filter: SafetyFilter) => {
    onChange(filter);
    if (document.activeElement instanceof HTMLElement) {
      document.activeElement.blur();
    }
  };

  return (
    <div className="dropdown dropdown-bottom dropdown-end">
      <button
        type="button"
        tabIndex={0}
        className="btn btn-ghost btn-sm h-8 min-h-8 gap-1.5 px-2.5 text-base-content/80 transition-colors hover:bg-base-content/10 hover:text-base-content focus-visible:outline focus-visible:outline-2 focus-visible:outline-primary focus-visible:outline-offset-2 active:translate-y-px"
        aria-label={t('safety_filter.label', 'Mod safety filter')}
        title={t('safety_filter.label', 'Mod safety filter')}
      >
        <ActiveIcon size={14} className={activeFilter.className} />
        <span className={`text-xs font-medium ${compact ? 'hidden lg:inline' : ''}`}>
          {t(`safety_filter.${value}`, value)}
        </span>
        <ChevronDown size={12} className="text-base-content/40" aria-hidden="true" />
      </button>

      <LiquidSurface
        liquidRole="overlay"
        className="dropdown-content z-[var(--workspace-layer-popover)] mt-2 w-48 rounded-box shadow-xl"
      >
        <ul tabIndex={0} className="menu w-full p-2">
          <li className="menu-title px-2 pb-1 text-[10px] font-medium uppercase tracking-widest text-base-content/40">
            <span>{t('safety_filter.label', 'Mod safety filter')}</span>
          </li>
          {FILTERS.map((filter) => {
            const FilterIcon = filter.icon;
            const isActive = filter.value === value;

            return (
              <li key={filter.value}>
                <button
                  type="button"
                  className={`flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm transition-colors ${
                    isActive
                      ? 'bg-primary/10 text-primary font-medium cursor-default'
                      : 'text-base-content/75 hover:bg-base-content/10 hover:text-base-content'
                  }`}
                  aria-current={isActive ? 'true' : undefined}
                  disabled={isActive}
                  onClick={() => selectFilter(filter.value)}
                >
                  <FilterIcon size={14} className={filter.className} />
                  {t(`safety_filter.${filter.value}`, filter.value)}
                </button>
              </li>
            );
          })}
        </ul>
      </LiquidSurface>
    </div>
  );
}
