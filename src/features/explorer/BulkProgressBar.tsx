import { useBulkProgress } from '../../hooks/useBulkProgress';

export default function BulkProgressBar() {
  const { active, label, current, total } = useBulkProgress();

  if (!active) return null;

  return (
    <div className="fixed top-20 left-1/2 -translate-x-1/2 z-100 w-full max-w-sm px-4 pointer-events-none">
      <div className="alert shadow-xl bg-base-100/95 backdrop-blur border border-base-content/10 flex flex-col items-stretch gap-2 p-3 rounded-lg animate-in fade-in slide-in-from-top-2 duration-200">
        <div className="flex justify-between items-center text-xs font-semibold tracking-wide">
          <span className="truncate pr-2">{label}</span>
          <span className="tabular-nums opacity-70 shrink-0">
            {Math.min(current, total)} / {total}
          </span>
        </div>
        <progress
          className="progress progress-primary w-full h-1.5 transition-all duration-300"
          value={current}
          max={total}
        ></progress>
      </div>
    </div>
  );
}
