import type { Ref } from 'react';
import { useTranslation } from 'react-i18next';
import DuplicateReport, {
  type DuplicateReportActionState,
  type DuplicateReportHandle,
} from './DuplicateReport';
import type { DedupScanProgress } from '../utils/dedupProgress';

export interface DedupFeatureProps extends DedupScanProgress {
  activeFilter?: 'all' | 'high' | 'medium' | 'low';
  gameId: string;
  reportRef?: Ref<DuplicateReportHandle>;
  onReportActionStateChange?: (state: DuplicateReportActionState) => void;
}

// ponytail: presentational only. The scan state lives in the page that owns the
// start/stop buttons, so there is nothing to mirror back up through a ref.
export default function DedupFeature({
  activeFilter,
  isScanning,
  totalFolders,
  scannedFolders,
  currentFolder,
  error,
  gameId,
  reportRef,
  onReportActionStateChange,
}: DedupFeatureProps) {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col gap-6">
      {error && (
        <div className="alert alert-error" role="alert">
          <span>{error}</span>
        </div>
      )}

      {/* Progress Overlay / Indicator */}
      {isScanning && (
        <div className="workspace-surface p-5">
          <div className="flex justify-between text-sm mb-3">
            <span className="font-bold text-primary flex items-center gap-2">
              <span
                className="inline-flex h-2.5 w-2.5 rounded-full bg-primary"
                aria-hidden="true"
              />
              {t('scanner:dedup.analyzing')}
            </span>
            <span className="font-mono text-base-content/60 font-medium">
              {t('scanner:dedup.progress', { scanned: scannedFolders, total: totalFolders })}
            </span>
          </div>
          <progress
            className="progress progress-primary h-4 w-full"
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
      {!isScanning && (
        <DuplicateReport
          ref={reportRef}
          activeFilter={activeFilter}
          gameId={gameId}
          showApplyAction={!reportRef}
          onActionStateChange={onReportActionStateChange}
        />
      )}
    </div>
  );
}
