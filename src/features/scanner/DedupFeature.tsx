import { useTranslation } from 'react-i18next';
import DuplicateReport from './components/DuplicateReport';

export interface DedupScanProgress {
  isScanning: boolean;
  totalFolders: number;
  scannedFolders: number;
  currentFolder: string;
}

export interface DedupFeatureProps extends DedupScanProgress {
  activeFilter?: 'all' | 'high' | 'medium' | 'low';
}

// ponytail: presentational only. The scan state lives in the page that owns the
// start/stop buttons, so there is nothing to mirror back up through a ref.
export default function DedupFeature({
  activeFilter,
  isScanning,
  totalFolders,
  scannedFolders,
  currentFolder,
}: DedupFeatureProps) {
  const { t } = useTranslation();

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
}
