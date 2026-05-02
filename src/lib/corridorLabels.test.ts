import { describe, expect, it } from 'vitest';
import { getCollectionDisplayName, getCorridorStateName } from './corridorLabels';

describe('corridorLabels', () => {
  const labels = {
    safeLabel: 'Unsaved SAFE Preset',
    unsafeLabel: 'Unsaved UNSAFE Preset',
  };

  it('uses the canonical safe unsaved label for safe collections', () => {
    expect(
      getCollectionDisplayName({
        name: '202603251217',
        isUnsaved: true,
        isSafe: true,
        labels,
      }),
    ).toBe('Unsaved SAFE Preset');
  });

  it('uses the canonical unsafe unsaved label for unsafe collections', () => {
    expect(
      getCollectionDisplayName({
        name: '202603251217',
        isUnsaved: true,
        isSafe: false,
        labels,
      }),
    ).toBe('Unsaved UNSAFE Preset');
  });

  it('keeps named collection labels unchanged', () => {
    expect(
      getCollectionDisplayName({
        name: 'My Build',
        isUnsaved: false,
        isSafe: true,
        labels,
      }),
    ).toBe('My Build');
  });

  it('does not generate timestamp-based fallback labels', () => {
    expect(
      getCorridorStateName({
        stateName: null,
        isUnsaved: true,
        isSafe: true,
        labels,
      }),
    ).toBe('Unsaved SAFE Preset');
  });
});
