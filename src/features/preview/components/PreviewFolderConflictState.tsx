import { AlertTriangle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { FolderNameConflictGroup } from '../../../core/tauri/bindings';

interface PreviewFolderConflictStateProps {
  conflict: FolderNameConflictGroup;
  onBack: () => void;
  onResolve: () => void;
}

export default function PreviewFolderConflictState({
  conflict,
  onBack,
  onResolve,
}: PreviewFolderConflictStateProps) {
  const { t } = useTranslation(['preview', 'common']);

  return (
    <aside className="mx-auto flex h-full w-full max-w-140 flex-col border-l border-base-content/5 bg-base-100/30 p-6 backdrop-blur-md">
      <button type="button" className="btn btn-sm btn-ghost self-start" onClick={onBack}>
        {t('common:actions.back')}
      </button>
      <div className="mt-8 rounded-xl border border-warning/40 bg-warning/10 p-5">
        <div className="flex items-start gap-3">
          <AlertTriangle className="mt-0.5 shrink-0 text-warning" size={22} aria-hidden="true" />
          <div className="min-w-0">
            <h2 className="font-semibold text-base-content">
              {t('preview:folder_conflict.title')}
            </h2>
            <p className="mt-1 text-sm text-base-content/70">
              {t('preview:folder_conflict.description')}
            </p>
          </div>
        </div>
        <ul className="mt-4 space-y-2">
          {conflict.candidates.map((candidate) => (
            <li
              key={candidate.path}
              className="rounded-lg border border-base-content/10 bg-base-100/60 px-3 py-2"
            >
              <div className="flex items-center justify-between gap-2 text-xs">
                <span className="font-medium text-base-content">{candidate.folder_name}</span>
                <span className={`badge badge-sm ${candidate.is_enabled ? 'badge-success' : ''}`}>
                  {t(
                    candidate.is_enabled
                      ? 'preview:folder_conflict.enabled'
                      : 'preview:folder_conflict.disabled',
                  )}
                </span>
              </div>
              <p className="mt-1 break-all text-xs text-base-content/50">{candidate.path}</p>
            </li>
          ))}
        </ul>
        <button type="button" className="btn btn-warning btn-sm mt-4 w-full" onClick={onResolve}>
          {t('preview:folder_conflict.resolve')}
        </button>
      </div>
    </aside>
  );
}
