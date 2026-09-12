import { useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';

import type {
  ConfidenceTier,
  DestinationMatchMethod,
} from '../../../shared/api/tauri/bindings.gen';

type Props = {
  score: number;
  tier: ConfidenceTier;
  destinationName: string;
  method: DestinationMatchMethod;
  manual: boolean;
  categoryWarning: boolean;
};

export function MatchScoreBadge({
  categoryWarning,
  destinationName,
  manual,
  method,
  score,
  tier,
}: Props) {
  const { t } = useTranslation('match_wizard');
  const anchorRef = useRef<HTMLButtonElement>(null);
  const [position, setPosition] = useState<{ left: number; top: number } | null>(null);

  const show = () => {
    const anchor = anchorRef.current;
    if (!anchor) return;
    const rect = anchor.getBoundingClientRect();
    const width = 256;
    const estimatedHeight = categoryWarning || manual ? 142 : 116;
    const left = Math.min(Math.max(8, rect.left), Math.max(8, window.innerWidth - width - 8));
    const top =
      rect.bottom + estimatedHeight + 8 <= window.innerHeight
        ? rect.bottom + 8
        : Math.max(8, rect.top - estimatedHeight - 8);
    setPosition({ left, top });
  };

  return (
    <div className="flex min-h-14 min-w-0 flex-col justify-center">
      <button
        ref={anchorRef}
        type="button"
        className={`badge badge-sm cursor-help ${confidenceClass(tier)}`}
        onMouseEnter={show}
        onMouseLeave={() => setPosition(null)}
        onFocus={show}
        onBlur={() => setPosition(null)}
      >
        {t('confidence_value', { value: score, label: t(`confidence.${tier}`) })}
      </button>
      <div className="mt-1 flex min-w-0 items-center gap-1.5">
        <p className="truncate text-[11px] text-base-content/55">{t(`match_methods.${method}`)}</p>
        {manual && (
          <span className="badge badge-primary badge-xs">{t('match_tooltip.manual')}</span>
        )}
      </div>

      {position &&
        createPortal(
          <aside
            role="tooltip"
            className="pointer-events-none fixed z-[1100] w-64 rounded-lg border border-base-300 bg-base-100 p-3 text-left text-xs font-normal text-base-content shadow-xl"
            style={position}
          >
            <p className="font-semibold text-base-content">{destinationName}</p>
            {manual && <p className="mt-0.5 text-primary">{t('match_tooltip.manual')}</p>}
            <p className="mt-2 text-base-content/70">{t(`match_methods.${method}`)}</p>
            {categoryWarning && (
              <p className="mt-1 text-warning">{t('match_tooltip.category_warning')}</p>
            )}
            <div className="mt-2 flex items-center justify-between border-t border-base-300 pt-2">
              <span className="text-base-content/55">{t('match_tooltip.score')}</span>
              <span className="font-semibold tabular-nums">{score}%</span>
            </div>
          </aside>,
          document.body,
        )}
    </div>
  );
}

function confidenceClass(tier: ConfidenceTier): string {
  if (tier === 'high') return 'badge-success';
  if (tier === 'medium') return 'badge-warning';
  if (tier === 'low') return 'badge-info';
  return 'badge-ghost';
}
