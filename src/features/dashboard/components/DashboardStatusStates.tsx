import { Gamepad2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useAppStore } from '../../../stores/useAppStore';

export function DashboardLoadingState() {
  return (
    <div className="h-full flex items-center justify-center bg-base-100">
      <span className="loading loading-spinner loading-lg text-primary"></span>
    </div>
  );
}

export function DashboardEmptyState() {
  const { t } = useTranslation(['dashboard']);
  const { setWorkspaceView } = useAppStore();

  return (
    <div className="flex flex-col items-center justify-center h-full bg-base-100 relative overflow-hidden">
      <div className="absolute inset-0 bg-linear-to-tr from-primary/5 via-transparent to-secondary/5 pointer-events-none" />
      <div className="z-10 text-center max-w-md px-6">
        <div className="mb-6 relative inline-block">
          <div className="absolute inset-0 bg-primary/20 blur-xl rounded-full" />
          <Gamepad2 size={64} className="text-primary relative z-10 mx-auto" />
        </div>
        <h1 className="text-3xl font-bold mb-3">{t('empty.title')}</h1>
        <p className="text-base-content/60 mb-8">{t('empty.subtitle')}</p>
        <button
          onClick={() => setWorkspaceView('settings')}
          className="btn btn-primary btn-lg gap-2"
        >
          <Gamepad2 size={20} />
          {t('empty.add_game')}
        </button>
      </div>
    </div>
  );
}
