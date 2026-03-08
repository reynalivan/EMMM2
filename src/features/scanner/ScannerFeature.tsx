import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { Play, ScanSearch } from 'lucide-react';
import { useScannerStore } from '../../stores/useScannerStore';
import { scanService } from '../../lib/services/scanService';
import { useActiveGame } from '../../hooks/useActiveGame';
import {
  useBulkToggle,
  useBulkDelete,
  useRenameMod,
  useAutoOrganizeMods,
} from '../../hooks/useFolders'; // Imported hooks
import type { ArchiveInfo } from '../../types/scanner';
import { toast } from '../../stores/useToastStore';

import ArchiveModal from './components/ArchiveModal';
import ScanOverlay from './components/ScanOverlay';
import ReviewTable from './components/ReviewTable';

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

  // #5: Track password errors for inline retry
  const [passwordError, setPasswordError] = useState<{ path: string; message: string } | null>(
    null,
  );

  // 2. Extract Mutation — uses shared extractArchiveBatch (A1)
  const extractMutation = useMutation({
    mutationFn: async ({
      paths,
      pwd,
      options,
    }: {
      paths: string[];
      pwd?: Record<string, string>;
      options?: {
        autoRename?: boolean;
        disableByDefault?: boolean;
        folderNames?: Record<string, string>;
        unpackNested?: boolean;
      };
    }) => {
      if (!activeGame) throw new Error('No active game config');

      // B3: Suppress watcher for the entire batch
      await invoke('set_watcher_suppression_cmd', { suppressed: true });

      try {
        setPasswordError(null);
        const result = await scanService.extractArchiveBatch(
          paths,
          archives,
          activeGame.mod_path,
          pwd ?? {},
          options,
        );

        if (result.error) {
          // #5: Password error → keep modal open for retry
          if (result.isPasswordError && result.failedPath) {
            setPasswordError({ path: result.failedPath, message: result.error });
            return; // don't throw — modal stays open
          }
          throw new Error(result.error);
        }
        // aborted is handled gracefully — no throw
      } finally {
        // B3: Always unsuppress + W2: scaled cooldown
        const { useAppStore } = await import('../../stores/useAppStore');
        const cooldown = Math.min(1000 + paths.length * 500, 5000);
        useAppStore.getState().setWatcherCooldown(Date.now() + cooldown);
        await invoke('set_watcher_suppression_cmd', { suppressed: false });
      }
    },
    onSuccess: () => {
      if (passwordError) return; // modal still open for retry
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
      setIsScanning(false);
      queryClient.invalidateQueries({ queryKey: ['mods'] });
      queryClient.invalidateQueries({ queryKey: ['conflicts'] });
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

    const { mod_path, game_type } = activeGame;
    scanMutation.mutate({ gameType: game_type, modsPath: mod_path });
  };

  const handleExtract = async (
    selectedPaths: string[],
    passwords: Record<string, string>,
    options?: {
      autoRename?: boolean;
      disableByDefault?: boolean;
      folderNames?: Record<string, string>;
    },
  ) => {
    extractMutation.mutate({ paths: selectedPaths, pwd: passwords, options });
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
            setPasswordError(null);
            handleStartScan();
          }}
          isExtracting={extractMutation.isPending}
          error={extractMutation.error ? String(extractMutation.error) : null}
          passwordError={passwordError}
          onStop={async () => {
            try {
              const { invoke } = await import('@tauri-apps/api/core');
              await invoke('abort_extraction_cmd');
            } catch (e) {
              console.error('Failed to abort scan extraction', e);
            }
          }}
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
                if (activeGame?.id) {
                  renameMod.mutate({ folderPath: path, newName, gameId: activeGame.id });
                }
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
      </div>
    </div>
  );
}
