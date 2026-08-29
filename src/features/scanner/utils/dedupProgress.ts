import type { DupScanEvent } from '@/entities/workspace/model/scanner';
import type { DedupScanProgress } from '../components/DedupFeature';

export function reduceDedupProgress(
  current: DedupScanProgress,
  event: DupScanEvent,
): DedupScanProgress {
  switch (event.event) {
    case 'started':
      return {
        ...current,
        isScanning: true,
        totalFolders: event.data.totalFolders,
        scannedFolders: 0,
        currentFolder: '',
        error: '',
      };
    case 'progress':
      return {
        ...current,
        scannedFolders: event.data.processedFolders,
        currentFolder: event.data.currentFolder,
      };
    case 'finished':
    case 'cancelled':
      return { ...current, isScanning: false };
    case 'failed':
      return { ...current, isScanning: false, error: event.data.message };
    case 'match':
      return current;
  }
}
