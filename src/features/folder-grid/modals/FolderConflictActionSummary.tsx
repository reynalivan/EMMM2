import { Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { FolderNameConflictGroup } from '../../../core/tauri/bindings';

interface Props {
  group: FolderNameConflictGroup | null;
  keepPath: string | null;
  drafts: Record<string, string>;
  submitting: boolean;
  onResolve: () => void;
}

export default function FolderConflictActionSummary({
  group,
  keepPath,
  drafts,
  submitting,
  onResolve,
}: Props) {
  const { t } = useTranslation('folder_grid');
  const renameCount = group ? Math.max(0, group.candidates.length - 1) : 0;

  return (
    <footer className="flex flex-col gap-3 border-t border-base-content/10 p-4 sm:flex-row sm:items-end sm:justify-between">
      <div className="min-w-0 flex-1">
        <p className="text-xs font-semibold text-base-content">
          {t('conflict_manager.action_plan')}
        </p>
        {group && keepPath && (
          <ul className="mt-1 max-h-20 space-y-1 overflow-y-auto text-[11px] text-base-content/60">
            {group.candidates.map((candidate) => (
              <li key={candidate.path} className="flex min-w-0 items-center gap-1.5">
                <span
                  className={`badge badge-xs shrink-0 ${candidate.path === keepPath ? 'badge-success' : 'badge-warning'}`}
                >
                  {candidate.path === keepPath
                    ? t('conflict_manager.keep_badge')
                    : t('conflict_manager.rename_badge')}
                </span>
                <span className="truncate" title={candidate.path}>
                  {candidate.path}
                </span>
                {candidate.path !== keepPath && (
                  <>
                    <span aria-hidden>→</span>
                    <span className="truncate font-medium text-base-content">
                      {drafts[candidate.path] ?? candidate.base_name}
                    </span>
                  </>
                )}
              </li>
            ))}
          </ul>
        )}
        <p className="mt-1 text-[11px] text-base-content/45">{t('conflict_manager.prefix_note')}</p>
      </div>
      <button
        className="btn btn-warning btn-sm shrink-0"
        disabled={submitting || !group}
        onClick={onResolve}
      >
        {submitting && <Loader2 size={15} className="animate-spin motion-reduce:animate-none" />}
        {t('conflict_manager.apply_renames', { count: renameCount })}
      </button>
    </footer>
  );
}
