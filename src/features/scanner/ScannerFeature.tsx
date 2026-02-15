import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AlertCircle, Play, ScanSearch } from 'lucide-react';
import { useScannerStore } from '../../stores/scannerStore';
import { scanService } from '../../services/scanService';
import { useActiveGame } from '../../hooks/useActiveGame';
import type { ArchiveInfo } from '../../types/scanner';

import ArchiveModal from '../../components/scanner/ArchiveModal';
import ScanOverlay from '../../components/scanner/ScanOverlay';
import ReviewTable from '../../components/scanner/ReviewTable';
import ConflictToast from '../../components/scanner/ConflictToast';
import type { ConflictInfo } from '../../types/scanner';

export default function ScannerFeature() {
  const queryClient = useQueryClient();
  const { activeGame } = useActiveGame();

  const {
    isScanning,
    setIsScanning,
    setTotalFolders,
    updateProgress,
    setStats,
    resetScanner,
    scanResults,
    setScanResults, // Added setScanResults
  } = useScannerStore();

  const [archives, setArchives] = useState<ArchiveInfo[]>([]);
  const [showArchiveModal, setShowArchiveModal] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [conflicts, setConflicts] = useState<ConflictInfo[]>([]);

  // 1. Detect Archives Mutation
  const detectMutation = useMutation({
    mutationFn: (path: string) => scanService.detectArchives(path),
    onSuccess: (foundArchives) => {
      if (foundArchives.length > 0) {
        setArchives(foundArchives);
        setShowArchiveModal(true);
      } else {
        // No archives? Start scan directly
        handleStartScan();
      }
    },
    onError: (err: unknown) => setErrorMessage(`Failed to detect archives: ${err}`),
  });

  // 2. Extract Mutation
  const extractMutation = useMutation({
    mutationFn: async ({
      paths,
      pwd,
      overwrite,
    }: {
      paths: string[];
      pwd?: string;
      overwrite?: boolean;
    }) => {
      if (!activeGame) throw new Error('No active game config');

      for (const archivePath of paths) {
        await scanService.extractArchive(archivePath, activeGame.mods_path, pwd, overwrite);
      }
    },
    onSuccess: () => {
      setShowArchiveModal(false);
      handleStartScan(); // Proceed to scan after extraction
    },
    onError: (err: unknown) => setErrorMessage(`Extraction failed: ${err}`),
  });

  // 3. Scan Mutation
  const scanMutation = useMutation({
    mutationFn: async ({ modsPath }: { modsPath: string }) => {
      resetScanner();
      setIsScanning(true);

      await scanService.startScan(modsPath, (event) => {
        switch (event.event) {
          case 'started':
            setTotalFolders(event.data.totalFolders);
            break;
          case 'progress':
            updateProgress(event.data.current, event.data.folderName);
            break;
          case 'matched':
            break;
          case 'finished':
            setStats(event.data.matched, event.data.unmatched);
            setStats(event.data.matched, event.data.unmatched);
            break;
        }
      });
    },
    onError: (err: unknown) => {
      console.error('Scan failed', err);
      console.error('Scan failed', err);
    },
    onSettled: async () => {
      // Always run conflict check after scan
      if (activeGame) {
        try {
          // Run conflict detection
          const conflicts = await scanService.detectConflictsInFolder(activeGame.mods_path);
          setConflicts(conflicts);
        } catch (e) {
          console.error('Conflict check failed', e);
        }
      }
      setIsScanning(false);
      queryClient.invalidateQueries({ queryKey: ['mods'] });
    },
  });

  const onScanClick = async () => {
    setErrorMessage(null);
    if (!activeGame) {
      setErrorMessage('No active game selected');
      return;
    }
    detectMutation.mutate(activeGame.mods_path);
  };

  const handleStartScan = async () => {
    if (!activeGame) return;

    setIsScanning(true);
    setScanResults([]);
    setConflicts([]);

    // Extract paths from config
    const { mods_path } = activeGame;

    scanMutation.mutate({ modsPath: mods_path });
  };

  const handleExtract = async (selectedPaths: string[], password?: string, overwrite?: boolean) => {
    extractMutation.mutate({ paths: selectedPaths, pwd: password, overwrite });
  };

  return (
    <div className="card bg-base-100 shadow-xl border border-base-300">
      <div className="card-body">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="card-title flex items-center gap-2">
              <ScanSearch className="w-5 h-5 text-primary" />
              Mod Scanner
            </h2>
            <p className="text-xs text-base-content/60">
              Detects new mods, archives, and updates existing records.
            </p>
          </div>
          <button
            className="btn btn-primary btn-sm"
            onClick={onScanClick}
            disabled={isScanning || detectMutation.isPending}
          >
            {detectMutation.isPending ? (
              <span className="loading loading-spinner text-primary-content"></span>
            ) : (
              <Play className="w-4 h-4 ml-1" />
            )}
            Start Scan
          </button>
        </div>

        {errorMessage && (
          <div role="alert" className="alert alert-error mt-4 text-sm py-2">
            <AlertCircle className="w-4 h-4" />
            <span>{errorMessage}</span>
          </div>
        )}

        {/* Components */}
        <ArchiveModal
          key={archives.length > 0 ? archives[0].path : 'empty'} // Force remount on new scan
          archives={archives}
          isOpen={showArchiveModal}
          onExtract={handleExtract}
          onSkip={() => {
            setShowArchiveModal(false);
            handleStartScan();
          }}
          isExtracting={extractMutation.isPending}
          error={extractMutation.error ? String(extractMutation.error) : null}
        />

        <ScanOverlay
          onCancel={async () => {
            try {
              await scanService.cancelScan();
            } catch (e) {
              console.error('Failed to cancel scan:', e);
            }
            // setIsScanning is handled by 'finished' event or error, but we can optimistically set false
            setIsScanning(false);
          }}
        />

        {/* Results Table (if scan finished) */}
        {!isScanning && scanResults.length > 0 && (
          <div className="mt-6 border-t border-base-200 pt-4">
            <ReviewTable
              data={scanResults}
              onOpenFolder={async (path) => {
                console.log('Open folder:', path);
              }}
              onRename={(path, newName) => {
                console.log('Rename', path, newName);
                // TODO: Call backend rename
              }}
            />
          </div>
        )}

        <ConflictToast conflicts={conflicts} onDismiss={() => setConflicts([])} />
      </div>
    </div>
  );
}
