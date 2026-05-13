import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { commands } from '../../lib/bindings';
import { type GameType } from '../../types/game';
import { Play, ScanSearch } from 'lucide-react';
import { useScannerStore } from '../../stores/useScannerStore';
import { scanService } from '../../lib/services/scanService';
import { useActiveGame } from '../../hooks/useActiveGame';
import { useTranslation } from 'react-i18next';
import { useRenameMod } from '../../hooks/useFolderCoreMutations';
import { useBulkToggle, useBulkDelete } from '../../hooks/useFolderMutations';
import type { ArchiveInfo } from '../../types/scanner';
import { toast } from '../../stores/useToastStore';
import type { ScanResultItem } from '../../types/scanner';
import { publishQueryScopes } from '../runtime-sync/queryRefresh';
import { withWatcherSuppression } from '../file-watcher/watcherSuppression';

import ArchiveModal from './components/ArchiveModal';
import ScanOverlay from './components/ScanOverlay';
import ReviewTable from './components/ReviewTable';

export default function ScannerFeature() {
  const { t } = useTranslation(['scanner', 'common']);
  const queryClient = useQueryClient();
  const { activeGame } = useActiveGame();

  // Action hooks
  const bulkToggle = useBulkToggle();
  const bulkDelete = useBulkDelete();
  const renameMod = useRenameMod();

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
    onError: (err: unknown) =>
      toast.error(`${t('scanner:extract.failed')}: ${err instanceof Error ? err.message : err}`),
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
      if (!activeGame) throw new Error(t('common:errors.no_active_game'));

      await withWatcherSuppression({ releaseDelayMs: null }, async () => {
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
      });
    },
    onSuccess: () => {
      if (passwordError) return; // modal still open for retry
      setShowArchiveModal(false);
      handleStartScan();
    },
    onError: (err: unknown) =>
      toast.error(
        `${t('scanner:extract.extraction_failed')}: ${err instanceof Error ? err.message : err}`,
      ),
  });

  // 3. Scan Mutation
  const scanMutation = useMutation({
    mutationFn: async ({ gameType, modsPath }: { gameType: GameType; modsPath: string }) => {
      resetScanner();
      setIsScanning(true);
      if (!activeGame) {
        throw new Error(t('common:errors.no_active_game'));
      }

      const previewItems = await scanService.runDeepmatchPreview(
        activeGame.id,
        gameType,
        modsPath,
        (event) => {
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
        },
      );

      return previewItems.map<ScanResultItem>((item) => ({
        path: item.folderPath,
        rawName: item.displayName,
        displayName: item.displayName,
        isDisabled: item.isDisabled,
        matchedAliasName: item.matchedAliasName,
        matchLevel: item.matchLevel as ScanResultItem['matchLevel'],
        confidence: item.confidence as ScanResultItem['confidence'],
        confidenceScore: item.confidenceScore,
        matchDetail: item.matchDetail,
        detectedSkin: item.detectedSkin,
        skinFolderName: null,
        thumbnailPath: item.thumbnailPath,
      }));
    },
    onSuccess: (results) => {
      setScanResults(results);
    },
    onError: (err: unknown) => {
      console.error('Scan failed', err);
      toast.error(`${t('common:errors.scan_failed')}: ${err instanceof Error ? err.message : err}`);
    },
    onSettled: async () => {
      setIsScanning(false);
      await publishQueryScopes(queryClient, ['folderStructure', 'conflicts']);
    },
  });

  const onScanClick = async () => {
    if (!activeGame) {
      toast.error(t('common:errors.no_active_game'));
      return;
    }
    detectMutation.mutate(activeGame.mod_path);
  };

  const handleStartScan = async () => {
    if (!activeGame) return;

    setIsScanning(true);
    setScanResults([]);

    const { mod_path, game_type } = activeGame;
    scanMutation.mutate({ gameType: game_type as unknown as GameType, modsPath: mod_path });
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
              {t('scanner:title')}
            </h2>
            <p className="text-xs text-base-content/60">{t('scanner:description')}</p>
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
            {t('scanner:start_button')}
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
              await commands.abortExtraction();
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
                if (!activeGame?.id) return;
                commands
                  .openInExplorer({ gameId: activeGame.id, path })
                  .catch((e) => console.error('Failed to open folder:', e));
              }}
              onRename={(path, newName) => {
                // Determine folderName from path for display, but hook expects full path?
                // useRenameMod expects { folderPath, newName }
                if (activeGame?.id) {
                  renameMod.mutate({ folderPath: path, newName, gameId: activeGame.id });
                }
              }}
              onBulkEnable={(paths) =>
                bulkToggle.mutate({ paths, enable: true, gameId: activeGame?.id || '' })
              }
              onBulkDisable={(paths) =>
                bulkToggle.mutate({ paths, enable: false, gameId: activeGame?.id || '' })
              }
              onBulkDelete={(paths) => {
                if (confirm(t('scanner:table.delete_confirm', { count: paths.length }))) {
                  bulkDelete.mutate({ paths, gameId: activeGame?.id });
                }
              }}
            />
          </div>
        )}
      </div>
    </div>
  );
}
