import { useCallback, useState } from 'react';
import { ChevronLeft, HardDrive, Play, StopCircle, EyeOff } from 'lucide-react';
import { useAppStore } from '../../stores/useAppStore';
import { useActiveGame } from '../../hooks/useActiveGame';
import { useCancelDedupScan, useIgnoredPairs, useStartDedupScan } from './hooks/useDedup';
import type { DupScanEvent } from '../../types/scanner';
import DedupFeature, { type DedupScanProgress } from './DedupFeature';
import { IgnoredPairsModal } from './components/IgnoredPairsModal';
import { useTranslation } from 'react-i18next';

const IDLE_PROGRESS: DedupScanProgress = {
  isScanning: false,
  totalFolders: 0,
  scannedFolders: 0,
  currentFolder: '',
};

export default function StorageOptimizerPage() {
  const { t } = useTranslation(['scanner']);
  const setWorkspaceView = useAppStore((state) => state.setWorkspaceView);
  const { activeGame } = useActiveGame();
  const startScan = useStartDedupScan();
  const cancelScan = useCancelDedupScan();
  const [progress, setProgress] = useState<DedupScanProgress>(IDLE_PROGRESS);
  const [showIgnoredModal, setShowIgnoredModal] = useState(false);
  const [activeTab, setActiveTab] = useState<'all' | 'high' | 'medium' | 'low'>('all');

  const { data: ignoredPairs } = useIgnoredPairs(activeGame?.id || '');
  const isScanning = progress.isScanning;

  const handleEvent = useCallback((event: DupScanEvent) => {
    switch (event.event) {
      case 'started':
        setProgress((current) => ({
          ...current,
          totalFolders: event.data.totalFolders,
          scannedFolders: 0,
        }));
        break;
      case 'progress':
        setProgress((current) => ({
          ...current,
          scannedFolders: event.data.processedFolders,
          currentFolder: event.data.currentFolder,
        }));
        break;
      case 'finished':
      case 'cancelled':
        setProgress((current) => ({ ...current, isScanning: false }));
        break;
    }
  }, []);

  const handleStartScan = useCallback(() => {
    if (!activeGame) return;
    setProgress({ ...IDLE_PROGRESS, isScanning: true });

    startScan.mutate(
      {
        gameId: activeGame.id,
        modsRoot: activeGame.mod_path,
        onEvent: handleEvent,
      },
      {
        onError: () => setProgress((current) => ({ ...current, isScanning: false })),
      },
    );
  }, [activeGame, handleEvent, startScan]);

  const handleCancelScan = useCallback(() => {
    cancelScan.mutate(undefined, {
      onSettled: () => setProgress((current) => ({ ...current, isScanning: false })),
    });
  }, [cancelScan]);

  return (
    <div className="h-full overflow-y-auto bg-base-100 animate-in fade-in duration-500">
      <div className="p-4 md:p-8 space-y-8 max-w-7xl mx-auto">
        {/* ── Header ─────────────────────────────────────────────────── */}
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-6 border-b border-base-300 pb-8">
          <div className="flex items-start gap-4">
            <button
              onClick={() => setWorkspaceView('dashboard')}
              className="btn btn-ghost btn-circle btn-md mt-1 hover:bg-primary/10 hover:text-primary transition-all duration-300"
              aria-label={t('common:actions.back')}
            >
              <ChevronLeft size={24} />
            </button>
            <div>
              <div className="flex items-center gap-3 mb-1">
                <HardDrive className="text-primary h-8 w-8" />
                <h1 className="text-2xl font-black tracking-tight text-base-content uppercase">
                  {t('scanner:optimizer.title')}
                </h1>
              </div>
              <p className="text-base-content/50 max-w-xl text-sm font-medium leading-relaxed">
                {t('scanner:optimizer.description')}
              </p>
            </div>
          </div>

          <div className="flex items-center gap-3 shrink-0">
            {ignoredPairs && ignoredPairs.length > 0 && (
              <button
                className="btn btn-ghost btn-md gap-2 px-4 h-12 rounded-xl text-primary hover:bg-primary/10"
                onClick={() => setShowIgnoredModal(true)}
              >
                <EyeOff size={18} />
                <span className="text-sm font-black uppercase tracking-wider">
                  {t('scanner:optimizer.ignored_button', { count: ignoredPairs.length })}
                </span>
              </button>
            )}

            {!isScanning ? (
              <button
                className="btn btn-primary btn-md shadow-xl shadow-primary/20 hover:scale-105 active:scale-95 transition-all gap-2 px-6 h-12 rounded-xl"
                onClick={handleStartScan}
              >
                <Play size={18} fill="currentColor" />
                <span className="text-sm font-black uppercase tracking-wider">
                  {t('scanner:optimizer.start_button')}
                </span>
              </button>
            ) : (
              <button
                className="btn btn-error btn-outline btn-md shadow-xl shadow-error/10 hover:bg-error hover:text-error-content hover:scale-105 active:scale-95 transition-all gap-2 px-6 h-12 rounded-xl"
                onClick={handleCancelScan}
              >
                <StopCircle size={18} />
                <span className="text-sm font-black uppercase tracking-wider">
                  {t('scanner:optimizer.stop_button')}
                </span>
              </button>
            )}
          </div>
        </div>

        {/* ── Filter Tabs ────────────────────────────────────────────── */}
        <div className="flex items-center justify-between gap-4">
          <div className="tabs tabs-boxed bg-base-300/50 p-1 w-fit border border-base-300 rounded-xl">
            {(['all', 'high', 'medium', 'low'] as const).map((tab) => (
              <button
                key={tab}
                className={`tab tab-sm md:tab-md font-bold uppercase tracking-wider px-8 transition-all duration-300 rounded-lg h-10 ${
                  activeTab === tab
                    ? 'tab-active bg-primary text-primary-content shadow-lg shadow-primary/20'
                    : 'hover:bg-base-content/5'
                }`}
                onClick={() => setActiveTab(tab)}
              >
                {t(`scanner:optimizer.tabs.${tab}`)}
              </button>
            ))}
          </div>

          <div className="text-xs font-bold uppercase tracking-widest text-base-content/30">
            {t('scanner:optimizer.filter_label')}
          </div>
        </div>

        {/* ── Main Feature Content ────────────────────────────────────── */}
        <div className="animate-in fade-in slide-in-from-bottom-8 duration-700 delay-200">
          <DedupFeature activeFilter={activeTab} {...progress} />
        </div>
      </div>

      {showIgnoredModal && activeGame && (
        <IgnoredPairsModal gameId={activeGame.id} onClose={() => setShowIgnoredModal(false)} />
      )}
    </div>
  );
}
