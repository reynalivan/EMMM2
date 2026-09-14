import { useTranslation } from 'react-i18next';
import WorkspacePanelSkeleton from '@/shared/ui/components/ui/WorkspacePanelSkeleton';

interface PreviewReadStateProps {
  errorMessage?: string | null;
  onRetry?: () => void;
}

export function PreviewLoadingState() {
  const { t } = useTranslation(['preview']);

  return (
    <div
      className="flex h-full w-full max-w-none flex-col border-l border-base-content/5 bg-base-100/85"
      role="status"
      aria-busy="true"
    >
      <WorkspacePanelSkeleton variant="preview" />
      <p className="px-6 pb-6 text-center text-sm text-base-content/60">
        {t('preview:loading.selected_mod')}
      </p>
    </div>
  );
}

export function PreviewErrorState({ errorMessage, onRetry }: PreviewReadStateProps) {
  const { t } = useTranslation(['preview']);

  return (
    <div className="flex h-full w-full max-w-none flex-col items-center justify-center border-l border-base-content/5 bg-base-100/85 p-6 pt-[var(--workspace-panel-content-inset)] text-center">
      <div className="max-w-sm text-base-content/60">
        <p className="text-xl font-bold text-base-content">{t('preview:errors.load_failed')}</p>
        <p className="mt-2 text-sm">{t('preview:errors.load_failed_description')}</p>
        {errorMessage ? <p className="mt-3 break-words text-xs">{errorMessage}</p> : null}
      </div>
      <button className="btn btn-outline btn-sm mt-5" onClick={onRetry}>
        {t('preview:actions.retry')}
      </button>
    </div>
  );
}
