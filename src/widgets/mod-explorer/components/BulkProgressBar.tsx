import { useTranslation } from 'react-i18next';
import { commands } from '../../../shared/api/tauri/bindings';
import { useBulkProgress } from '../hooks/useBulkProgress';

export default function BulkProgressBar() {
  const { t } = useTranslation(['common']);
  const { active, label, current, total } = useBulkProgress();

  if (!active) return null;

  return (
    <div className="pointer-events-none fixed left-1/2 top-[calc(var(--workspace-topbar-height)+0.75rem)] z-100 w-full max-w-sm -translate-x-1/2 px-4">
      <div className="alert flex flex-col items-stretch gap-2 rounded-lg border border-base-content/10 bg-base-100/95 p-3 backdrop-blur animate-in fade-in slide-in-from-top-2 duration-200">
        <div className="flex justify-between items-center text-xs font-semibold tracking-wide">
          <span className="truncate pr-2">{t(label, { count: total, defaultValue: label })}</span>
          <span className="tabular-nums opacity-70 shrink-0">
            {Math.min(current, total)} / {total}
          </span>
        </div>
        <progress
          className="progress progress-primary h-1.5 w-full transition-[color] duration-150"
          value={current}
          max={total}
        ></progress>
        {/* The wrapper is click-through so the bar never blocks the grid; only
            the button opts back in. Cancelling is idempotent, so no local
            pending state — the flag is already set on a second click. */}
        <button
          type="button"
          className="btn btn-ghost btn-xs self-end pointer-events-auto"
          onClick={() => void commands.bulkCancel()}
        >
          {t('common:actions.cancel')}
        </button>
      </div>
    </div>
  );
}
