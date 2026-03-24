import { useState, useCallback, useImperativeHandle, forwardRef } from 'react';
import { useTranslation } from 'react-i18next';
import { useActiveGame } from '../../hooks/useActiveGame';
import { useStartDedupScan, useCancelDedupScan } from './hooks/useDedup';
import type { DupScanEvent } from '../../types/scanner';
import DuplicateReport from './components/DuplicateReport';

export interface DedupFeatureRef {
  startScan: () => void;
  cancelScan: () => void;
  isScanning: boolean;
}

export interface DedupFeatureProps {
  activeFilter?: 'all' | 'high' | 'medium' | 'low';
}

const DedupFeature = forwardRef<DedupFeatureRef, DedupFeatureProps>(({ activeFilter }, ref) => {
  const { t } = useTranslation();
  const { activeGame } = useActiveGame();
  const startScan = useStartDedupScan();
  const cancelScan = useCancelDedupScan();

  const [isScanning, setIsScanning] = useState(false);
  const [totalFolders, setTotalFolders] = useState(0);
  const [scannedFolders, setScannedFolders] = useState(0);
  const [currentFolder, setCurrentFolder] = useState('');

  const handleEvent = useCallback((event: DupScanEvent) => {
    switch (event.event) {
      case 'started':
        setTotalFolders(event.data.totalFolders);
        setScannedFolders(0);
        break;
      case 'progress':
        setScannedFolders(event.data.processedFolders);
        setCurrentFolder(event.data.currentFolder);
        break;
      case 'finished':
      case 'cancelled':
        setIsScanning(false);
        break;
    }
  }, []);

  const handleStartScan = useCallback(() => {
    if (!activeGame) return;
    setIsScanning(true);
    setScannedFolders(0);
    setTotalFolders(0);
    setCurrentFolder('');

    startScan.mutate(
      {
        gameId: activeGame.id,
        modsRoot: activeGame.mod_path,
        onEvent: handleEvent,
      },
      {
        onError: () => setIsScanning(false),
      },
    );
  }, [activeGame, handleEvent, startScan]);

  const handleCancelScan = useCallback(() => {
    cancelScan.mutate(undefined, {
      onSettled: () => setIsScanning(false),
    });
  }, [cancelScan]);

  useImperativeHandle(ref, () => ({
    startScan: handleStartScan,
    cancelScan: handleCancelScan,
    isScanning,
  }));

  return (
    <div className="flex flex-col gap-6">
      {/* Progress Overlay / Indicator */}
      {isScanning && (
        <div className="p-6 rounded-2xl bg-base-200/50 border border-base-content/10 animate-in fade-in slide-in-from-top-4 duration-500">
          <div className="flex justify-between text-sm mb-3">
            <span className="font-bold text-primary flex items-center gap-2">
              <span className="relative flex h-3 w-3">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-primary opacity-75"></span>
                <span className="relative inline-flex rounded-full h-3 w-3 bg-primary"></span>
              </span>
              {t('scanner:dedup.analyzing')}
            </span>
            <span className="font-mono text-base-content/60 font-medium">
              {t('scanner:dedup.progress', { scanned: scannedFolders, total: totalFolders })}
            </span>
          </div>
          <progress
            className="progress progress-primary w-full h-4 shadow-sm"
            value={scannedFolders}
            max={Math.max(1, totalFolders)}
          />
          {currentFolder && (
            <div className="mt-3 text-[10px] sm:text-xs font-mono truncate text-base-content/40 bg-base-300/30 px-2 py-1 rounded">
              {t('scanner:dedup.current', { folder: currentFolder })}
            </div>
          )}
        </div>
      )}

      {/* Duplicate Report Component renders below */}
      {!isScanning && <DuplicateReport activeFilter={activeFilter} />}
    </div>
  );
});

DedupFeature.displayName = 'DedupFeature';
export default DedupFeature;
