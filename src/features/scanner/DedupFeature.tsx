import { useState, useCallback } from 'react';
import { Play, StopCircle, HardDrive } from 'lucide-react';
import { useActiveGame } from '../../hooks/useActiveGame';
import { useStartDedupScan, useCancelDedupScan } from '../../hooks/useDedup';
import type { DupScanEvent } from '../../types/dedup';
import DuplicateReport from './components/DuplicateReport';

export default function DedupFeature() {
  const { activeGame } = useActiveGame();
  const startScan = useStartDedupScan();
  const cancelScan = useCancelDedupScan();

  const [isScanning, setIsScanning] = useState(false);
  const [totalFolders, setTotalFolders] = useState(0);
  const [scannedFolders, setScannedFolders] = useState(0);
  const [currentFolder, setCurrentFolder] = useState('');

  const handleEvent = useCallback((event: DupScanEvent) => {
    switch (event.event) {
      case 'Started':
        setTotalFolders(event.data.totalFolders);
        setScannedFolders(0);
        break;
      case 'Progress':
        setScannedFolders(event.data.processedFolders);
        setCurrentFolder(event.data.currentFolder);
        break;
      case 'Finished':
      case 'Cancelled':
        setIsScanning(false);
        break;
    }
  }, []);

  const handleStartScan = () => {
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
  };

  const handleCancelScan = () => {
    cancelScan.mutate(undefined, {
      onSettled: () => setIsScanning(false),
    });
  };

  return (
    <div className="flex flex-col gap-6">
      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body">
          <div className="flex items-start justify-between">
            <div>
              <h2 className="card-title text-xl flex items-center gap-2">
                <HardDrive className="text-secondary" size={24} />
                Storage Optimizer (Duplicate Scanner)
              </h2>
              <p className="text-sm opacity-70 mt-1 max-w-2xl">
                Scan your entire mod library to find duplicate mods using advanced structural
                similarities and high-speed BLAKE3 content hashing. Reclaiming disk space has never
                been safer or easier.
              </p>
            </div>
            {!isScanning ? (
              <button
                className="btn btn-primary shadow-lg shadow-primary/20 shrink-0"
                onClick={handleStartScan}
                disabled={!activeGame || startScan.isPending}
              >
                <Play size={18} fill="currentColor" />
                Start Full Scan
              </button>
            ) : (
              <button
                className="btn btn-error btn-outline shadow-lg shrink-0"
                onClick={handleCancelScan}
                disabled={cancelScan.isPending}
              >
                <StopCircle size={18} />
                Cancel Scan
              </button>
            )}
          </div>

          {/* Progress Overlay / Indicator */}
          {isScanning && (
            <div className="mt-6 p-4 rounded-xl bg-base-300/50 border border-base-content/10">
              <div className="flex justify-between text-sm mb-2">
                <span className="font-semibold text-primary animate-pulse">
                  Hashing files & analyzing structures...
                </span>
                <span className="font-mono text-base-content/60">
                  {scannedFolders} / {totalFolders} folders
                </span>
              </div>
              <progress
                className="progress progress-primary w-full h-3"
                value={scannedFolders}
                max={Math.max(1, totalFolders)}
              />
              {currentFolder && (
                <div className="mt-2 text-xs font-mono truncate text-base-content/50">
                  Processing: {currentFolder}
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      <div className="divider my-0"></div>

      {/* Duplicate Report Component renders below */}
      {!isScanning && <DuplicateReport />}
    </div>
  );
}
