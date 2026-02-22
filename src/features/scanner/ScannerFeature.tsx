import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { dirname, basename, join } from '@tauri-apps/api/path';
import { mkdir, exists, rename } from '@tauri-apps/plugin-fs';
import { Play, ScanSearch } from 'lucide-react';
import { useScannerStore } from '../../stores/scannerStore';
import { scanService } from '../../services/scanService';
import { useActiveGame } from '../../hooks/useActiveGame';
import {
  useBulkToggle,
  useBulkDelete,
  useRenameMod,
  useAutoOrganizeMods,
} from '../../hooks/useFolders'; // Imported hooks
import type { ArchiveInfo } from '../../types/scanner';
import { toast } from '../../stores/useToastStore';

import ArchiveModal from '../../components/scanner/ArchiveModal';
import ScanOverlay from '../../components/scanner/ScanOverlay';
import ReviewTable from '../../components/scanner/ReviewTable';
import ConflictToast from '../../components/scanner/ConflictToast';
import type { ConflictInfo } from '../../types/scanner';

export default function ScannerFeature() {
  const queryClient = useQueryClient();
  const { activeGame } = useActiveGame();

  // Hooks for actions
  const bulkToggle = useBulkToggle();
  const bulkDelete = useBulkDelete();
  const renameMod = useRenameMod();
  const autoOrganize = useAutoOrganizeMods();

  const {
    isScanning,
    setIsScanning,
    setTotalFolders,
    updateProgress,
    setStats,
    resetScanner,
    scanResults,
    setScanResults,
  } = useScannerStore();

  const [archives, setArchives] = useState<ArchiveInfo[]>([]);
  const [showArchiveModal, setShowArchiveModal] = useState(false);
  const [conflicts, setConflicts] = useState<ConflictInfo[]>([]);

  // 1. Detect Archives Mutation
  const detectMutation = useMutation({
    mutationFn: (path: string) => scanService.detectArchives(path),
    onSuccess: (foundArchives) => {
      if (foundArchives.length > 0) {
        setArchives(foundArchives);
        setShowArchiveModal(true);
      } else {
        handleStartScan();
      }
    },
    onError: (err: unknown) => toast.error(`Failed to detect archives: ${err}`),
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
        const result = await scanService.extractArchive(
          archivePath,
          activeGame.mod_path,
          pwd,
          overwrite,
        );
        if (result.success) {
          try {
            const dir = await dirname(archivePath);
            const base = await basename(archivePath);
            const extractedDir = await join(dir, '.extracted');
            if (!(await exists(extractedDir))) {
              await mkdir(extractedDir, { recursive: true });
            }
            await rename(archivePath, await join(extractedDir, base));
          } catch (e) {
            console.warn(`Failed to move extracted archive ${archivePath}:`, e);
          }
        } else {
          throw new Error(result.error ?? 'Unknown error during extraction');
        }
      }
    },
    onSuccess: () => {
      setShowArchiveModal(false);
      handleStartScan();
    },
    onError: (err: unknown) => toast.error(`Extraction failed: ${err}`),
  });

  // 3. Scan Mutation
  const scanMutation = useMutation({
    mutationFn: async ({ gameType, modsPath }: { gameType: string; modsPath: string }) => {
      resetScanner();
      setIsScanning(true);

      await scanService.startScan(gameType, modsPath, (event) => {
        switch (event.event) {
          case 'started':
            setTotalFolders(event.data.totalFolders);
            break;
          case 'progress':
            updateProgress(event.data.current, event.data.folderName, event.data.etaMs ?? 0);
            break;
          case 'matched':
            break;
          case 'finished':
            setStats(event.data.matched, event.data.unmatched);
            break;
        }
      });
    },
    onError: (err: unknown) => {
      console.error('Scan failed', err);
      toast.error(`Scan failed: ${err}`);
    },
    onSettled: async () => {
      if (activeGame) {
        try {
          const conflicts = await scanService.detectConflictsInFolder(activeGame.mod_path);
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
    if (!activeGame) {
      toast.error('No active game selected');
      return;
    }
    detectMutation.mutate(activeGame.mod_path);
  };

  const handleStartScan = async () => {
    if (!activeGame) return;

    setIsScanning(true);
    setScanResults([]);
    setConflicts([]);

    const { mod_path, game_type } = activeGame;
    scanMutation.mutate({ gameType: game_type, modsPath: mod_path });
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

        {/* Components */}
        <ArchiveModal
          key={archives.length > 0 ? archives[0].path : 'empty'}
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
            setIsScanning(false);
          }}
        />

        {/* Results Table (if scan finished) */}
        {!isScanning && scanResults.length > 0 && (
          <div className="mt-6 border-t border-base-200 pt-4">
            <ReviewTable
              data={scanResults}
              onOpenFolder={(path) => {
                invoke('open_in_explorer', { path }).catch((e) =>
                  console.error('Failed to open folder:', e),
                );
              }}
              onRename={(path, newName) => {
                // Determine folderName from path for display, but hook expects full path?
                // useRenameMod expects { folderPath, newName }
                renameMod.mutate({ folderPath: path, newName });
              }}
              onBulkEnable={(paths) => bulkToggle.mutate({ paths, enable: true })}
              onBulkDisable={(paths) => bulkToggle.mutate({ paths, enable: false })}
              onBulkDelete={(paths) => {
                if (confirm(`Are you sure you want to delete ${paths.length} mods?`)) {
                  bulkDelete.mutate({ paths, gameId: activeGame?.id });
                }
              }}
              onAutoOrganize={async (paths) => {
                if (!activeGame) return;
                try {
                  const dbJson = await scanService.getMasterDb(activeGame.game_type);
                  autoOrganize.mutate({
                    paths,
                    targetRoot: activeGame.mod_path,
                    dbJson,
                  });
                } catch (e) {
                  toast.error(`Failed to load DB for auto-organize: ${e}`);
                }
              }}
            />
          </div>
        )}

        <ConflictToast conflicts={conflicts} onDismiss={() => setConflicts([])} />
      </div>
    </div>
  );
}
