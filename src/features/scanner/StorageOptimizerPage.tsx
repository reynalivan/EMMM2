import { useCallback, useRef, useState } from 'react';
import { Play, StopCircle, EyeOff } from 'lucide-react';
import { useActiveGame } from '@/entities/game';
import { useCancelDedupScan, useIgnoredPairs, useStartDedupScan } from './hooks/useDedup';
import DedupFeature from './components/DedupFeature';
import {
  DuplicateReportApplyButton,
  type DuplicateReportActionState,
  type DuplicateReportHandle,
} from './components/DuplicateReport';
import { IgnoredPairsModal } from './components/IgnoredPairsModal';
import { useTranslation } from 'react-i18next';
import { IDLE_DEDUP_SCAN_PROGRESS, type DedupScanProgress } from './utils/dedupProgress';
import { useDedupScanStore } from './stores/useDedupScanStore';
import {
  WorkspaceContextBar,
  WorkspacePageContent,
  WorkspacePageFrame,
} from '@/shared/ui/components/layout/WorkspacePageFrame';
import { LiquidSurface } from '@/shared/ui/liquid';
import { TopBarActionsPortal } from '@/shared/ui/components/layout/TopBarActionsPortal';

export default function StorageOptimizerPage() {
  const { t } = useTranslation(['scanner']);
  const { activeGame } = useActiveGame();
  const startScan = useStartDedupScan();
  const cancelScan = useCancelDedupScan();
  const scanGameId = useDedupScanStore((state) => state.gameId);
  const storedProgress = useDedupScanStore((state) => state.progress);
  const [showIgnoredModal, setShowIgnoredModal] = useState(false);
  const [activeTab, setActiveTab] = useState<'all' | 'high' | 'medium' | 'low'>('all');
  const duplicateReportRef = useRef<DuplicateReportHandle>(null);
  const [duplicateActionState, setDuplicateActionState] = useState<DuplicateReportActionState>({
    selectionCount: 0,
    isApplying: false,
  });

  const { data: ignoredPairs } = useIgnoredPairs(activeGame?.id || '');
  const activeGameId = activeGame?.id ?? null;
  const progress: DedupScanProgress =
    scanGameId === activeGameId ? storedProgress : IDLE_DEDUP_SCAN_PROGRESS;
  const isScanning = storedProgress.isScanning;

  const handleDuplicateActionStateChange = useCallback((next: DuplicateReportActionState) => {
    setDuplicateActionState((current) =>
      current.selectionCount === next.selectionCount && current.isApplying === next.isApplying
        ? current
        : next,
    );
  }, []);

  const handleStartScan = useCallback(() => {
    if (!activeGame || isScanning) return;
    startScan.mutate({ gameId: activeGame.id, modsRoot: activeGame.mod_path });
  }, [activeGame, isScanning, startScan]);

  const handleCancelScan = useCallback(() => {
    if (!scanGameId || !isScanning || cancelScan.isPending) return;
    cancelScan.mutate();
  }, [cancelScan, isScanning, scanGameId]);

  return (
    <WorkspacePageFrame
      context={
        <WorkspaceContextBar
          description={
            <div className="flex w-full flex-wrap items-center gap-3">
              <StorageFilterTabs activeTab={activeTab} onChange={setActiveTab} />
              <DuplicateReportApplyButton
                className="ml-auto"
                selectionCount={duplicateActionState.selectionCount}
                isApplying={duplicateActionState.isApplying}
                onApply={() => duplicateReportRef.current?.requestApply()}
              />
            </div>
          }
        />
      }
    >
      <TopBarActionsPortal>
        {ignoredPairs && ignoredPairs.length > 0 && (
          <button
            className="btn btn-ghost btn-sm btn-square"
            onClick={() => setShowIgnoredModal(true)}
            title={t('scanner:optimizer.ignored_button', { count: ignoredPairs.length })}
          >
            <EyeOff size={18} />
          </button>
        )}
        {!isScanning ? (
          <button
            className="btn btn-primary btn-sm gap-2 whitespace-nowrap"
            onClick={handleStartScan}
          >
            <Play size={16} fill="currentColor" />
            <span>{t('scanner:optimizer.start_button')}</span>
          </button>
        ) : (
          <button
            className="btn btn-error btn-outline btn-sm gap-2 whitespace-nowrap"
            onClick={handleCancelScan}
            disabled={cancelScan.isPending}
          >
            <StopCircle size={16} />
            <span>{t('scanner:optimizer.stop_button')}</span>
          </button>
        )}
      </TopBarActionsPortal>
      <WorkspacePageContent>
        {/* ── Filter Tabs ────────────────────────────────────────────── */}
        <DedupFeature
          activeFilter={activeTab}
          gameId={activeGame?.id ?? ''}
          reportRef={duplicateReportRef}
          onReportActionStateChange={handleDuplicateActionStateChange}
          {...progress}
        />
      </WorkspacePageContent>

      {showIgnoredModal && activeGame && (
        <IgnoredPairsModal gameId={activeGame.id} onClose={() => setShowIgnoredModal(false)} />
      )}
    </WorkspacePageFrame>
  );
}

function StorageFilterTabs({
  activeTab,
  onChange,
}: {
  activeTab: 'all' | 'high' | 'medium' | 'low';
  onChange: (tab: 'all' | 'high' | 'medium' | 'low') => void;
}) {
  const { t } = useTranslation(['scanner']);

  return (
    <div role="tablist" aria-label={t('scanner:optimizer.filter_label')}>
      <LiquidSurface
        liquidRole="control"
        className="rounded-[var(--radius-box)]"
        contentClassName="flex gap-1 p-1"
      >
        {(['all', 'high', 'medium', 'low'] as const).map((tab) => {
          const active = activeTab === tab;
          return (
            <button
              key={tab}
              role="tab"
              aria-selected={active}
              className={`min-h-8 whitespace-nowrap rounded-[calc(var(--radius-box)-0.25rem)] px-3 text-sm font-medium transition-[background-color,color] duration-150 sm:px-4 ${
                active
                  ? 'bg-base-content/[0.08] text-base-content'
                  : 'text-base-content/55 hover:bg-base-content/[0.05] hover:text-base-content'
              }`}
              onClick={() => onChange(tab)}
            >
              {t(`scanner:optimizer.tabs.${tab}`)}
            </button>
          );
        })}
      </LiquidSurface>
    </div>
  );
}
