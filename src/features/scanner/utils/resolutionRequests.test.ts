import { describe, expect, it } from 'vitest';
import type { DupScanGroup, DuplicateSelection } from '@/entities/workspace';
import { buildResolutionRequests } from './resolutionRequests';

function group(id: string, paths: string[], confidenceScore = 100): DupScanGroup {
  return {
    groupId: id,
    confidenceScore,
    matchReason: 'test',
    isUnsafe: false,
    signals: [],
    members: paths.map((folderPath) => ({
      modId: null,
      version: 1,
      folderPath,
      displayName: folderPath,
      totalSizeBytes: 0,
      fileCount: 0,
      isSafe: true,
      confidenceScore,
      signals: [],
    })),
  } as unknown as DupScanGroup;
}

describe('buildResolutionRequests', () => {
  it('keeps one member by trashing every other member', () => {
    const selections = new Map<string, DuplicateSelection>([
      ['g1', { type: 'Keep', targetPath: 'a' }],
    ]);

    expect(buildResolutionRequests(selections, [group('g1', ['a', 'b', 'c'])])).toEqual([
      { groupId: 'g1', action: 'keepA', folderA: 'a', folderB: 'b' },
      { groupId: 'g1', action: 'keepA', folderA: 'a', folderB: 'c' },
    ]);
  });

  it('whitelists every pair when the group is ignored', () => {
    const selections = new Map<string, DuplicateSelection>([['g1', { type: 'Ignore' }]]);

    expect(buildResolutionRequests(selections, [group('g1', ['a', 'b', 'c'])])).toEqual([
      { groupId: 'g1', action: 'ignore', folderA: 'a', folderB: 'b' },
      { groupId: 'g1', action: 'ignore', folderA: 'a', folderB: 'c' },
      { groupId: 'g1', action: 'ignore', folderA: 'b', folderB: 'c' },
    ]);
  });

  it('skips groups the user did not decide on', () => {
    const selections = new Map<string, DuplicateSelection>([['g1', null]]);

    expect(
      buildResolutionRequests(selections, [group('g1', ['a', 'b']), group('g2', ['c', 'd'])]),
    ).toEqual([]);
  });

  it('drops a Keep whose target is no longer part of the group', () => {
    const selections = new Map<string, DuplicateSelection>([
      ['g1', { type: 'Keep', targetPath: 'gone' }],
    ]);

    expect(buildResolutionRequests(selections, [group('g1', ['a', 'b'])])).toEqual([]);
  });

  it('emits nothing for a single-member group kept as-is', () => {
    const selections = new Map<string, DuplicateSelection>([
      ['g1', { type: 'Keep', targetPath: 'a' }],
    ]);

    expect(buildResolutionRequests(selections, [group('g1', ['a'])])).toEqual([]);
  });

  it('does not build destructive requests for an inexact group', () => {
    const selections = new Map<string, DuplicateSelection>([
      ['g1', { type: 'Keep', targetPath: 'a' }],
    ]);

    expect(buildResolutionRequests(selections, [group('g1', ['a', 'b'], 85)])).toEqual([]);
  });
});
