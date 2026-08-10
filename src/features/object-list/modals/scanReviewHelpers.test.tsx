import { describe, expect, it } from 'vitest';
import type { ScanPreviewItem } from '../../../types/scanner';
import { groupByConfidence, type MasterDbEntry } from './scanReviewHelpers';

function previewItem(folderPath: string, confidence: string): ScanPreviewItem {
  return {
    folderPath,
    displayName: folderPath,
    isDisabled: false,
    matchedEntryKey: null,
    matchedAliasName: null,
    matchLevel: 'NoMatch',
    confidence,
    confidenceScore: 0,
    matchDetail: null,
    detectedSkin: null,
    objectType: null,
    thumbnailPath: null,
    tagsJson: null,
    metadataJson: null,
    alreadyInDb: false,
    alreadyMatched: false,
    scoredCandidates: [],
    hashDbJson: null,
    customSkinsJson: null,
    dbThumbnail: null,
  };
}

describe('groupByConfidence', () => {
  it('orders tiers, drops empty groups, and promotes overrides to Manual', () => {
    const low = previewItem('low', 'Low');
    const none = previewItem('none', 'None');
    const override: MasterDbEntry = {
      matched_entry_key: 'manual',
      name: 'Manual Target',
      object_type: 'Character',
      tags: [],
      metadata: null,
      thumbnail_path: null,
    };

    const groups = groupByConfidence([low, none], { [none.folderPath]: override });

    expect(groups.map((group) => group.tier)).toEqual(['Manual', 'Low']);
    expect(groups[0]?.items).toEqual([none]);
    expect(groups[1]?.items).toEqual([low]);
  });
});
