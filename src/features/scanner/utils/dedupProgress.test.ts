import { describe, expect, it } from 'vitest';
import type { DupScanEvent } from '@/entities/workspace';
import { reduceDedupProgress } from './dedupProgress';

describe('reduceDedupProgress', () => {
  it('stops the scan and exposes the backend message on failure', () => {
    const event: DupScanEvent = {
      event: 'failed',
      data: {
        scanId: 'scan-1',
        processedFolders: 3,
        totalFolders: 10,
        message: 'Unable to read one mod root',
      },
    };

    expect(
      reduceDedupProgress(
        {
          isScanning: true,
          totalFolders: 10,
          scannedFolders: 3,
          currentFolder: 'Mod A',
          error: '',
        },
        event,
      ),
    ).toEqual({
      isScanning: false,
      totalFolders: 10,
      scannedFolders: 3,
      currentFolder: 'Mod A',
      error: 'Unable to read one mod root',
    });
  });
});
