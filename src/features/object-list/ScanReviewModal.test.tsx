import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import ScanReviewModal from './ScanReviewModal';
import type { ScanPreviewItem } from '../../types/scanner';

vi.mock('./ScanReviewRow', () => ({
  default: ({ item }: { item: ScanPreviewItem }) => (
    <tr data-testid="scan-review-row">
      <td>{item.displayName}</td>
    </tr>
  ),
}));

const previewItem: ScanPreviewItem = {
  folderPath: 'E:\\Mods\\.emmm_temp\\Temp Mod',
  displayName: 'Temp Mod',
  isDisabled: false,
  matchedEntryKey: 'keqing',
  matchedAliasName: 'Keqing',
  matchLevel: 'AutoMatched',
  confidence: 'High',
  confidenceScore: 92,
  matchDetail: 'matched by name',
  detectedSkin: null,
  objectType: 'Character',
  thumbnailPath: null,
  tagsJson: null,
  metadataJson: null,
  alreadyInDb: false,
  alreadyMatched: false,
  scoredCandidates: [],
  hashDbJson: null,
  customSkinsJson: null,
  dbThumbnail: null,
  moveFromTemp: true,
};

describe('ScanReviewModal', () => {
  it('preserves temp move flag when confirming preview items', () => {
    const onConfirm = vi.fn();

    render(
      <ScanReviewModal
        activeGame={null}
        open={true}
        items={[previewItem]}
        masterDbEntries={[]}
        isCommitting={false}
        onConfirm={onConfirm}
        onClose={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: /Confirm 1 Items/i }));

    expect(onConfirm).toHaveBeenCalledWith([
      expect.objectContaining({
        folderPath: previewItem.folderPath,
        moveFromTemp: true,
      }),
    ]);
  });
});
