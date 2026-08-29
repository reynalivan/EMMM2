import { describe, expect, it } from 'vitest';
import type { ConflictInfo } from '@/entities/workspace/model/scanner';
import {
  buildConflictKey,
  chooseConflictWinner,
  setModDecision,
  summarizeConflictResolution,
} from './conflictResolution';

function conflict(hash: string, modPaths: string[]): ConflictInfo {
  return {
    hash,
    section_name: 'TextureOverrideBody',
    mod_paths: modPaths,
    is_active: true,
    kind: 'resource_hash',
    certainty: 'potential',
    has_conditional_evidence: false,
    evidence: [],
  };
}

describe('conflict resolution decisions', () => {
  it('builds a stable key independent of participant order', () => {
    const left = conflict('aaaaaaaa', ['E:/Mods/ModA', 'E:/Mods/ModB']);
    const right = conflict('aaaaaaaa', ['E:/Mods/ModB', 'E:/Mods/ModA']);

    expect(buildConflictKey(left)).toBe(buildConflictKey(right));
  });

  it('keeps one winner and disables the other participants without mutating prior state', () => {
    const existing = new Map([['E:/Mods/Unrelated', 'keep'] as const]);
    const selected = chooseConflictWinner(
      existing,
      conflict('aaaaaaaa', ['E:/Mods/ModA', 'E:/Mods/ModB']),
      'E:/Mods/ModA',
    );

    expect(existing).toEqual(new Map([['E:/Mods/Unrelated', 'keep']]));
    expect(selected).toEqual(
      new Map([
        ['E:/Mods/Unrelated', 'keep'],
        ['E:/Mods/ModA', 'keep'],
        ['E:/Mods/ModB', 'disable'],
      ]),
    );
  });

  it('applies one mod decision consistently across overlapping conflict groups', () => {
    const conflicts = [
      conflict('aaaaaaaa', ['E:/Mods/ModA', 'E:/Mods/ModB']),
      conflict('bbbbbbbb', ['E:/Mods/ModB', 'E:/Mods/ModC']),
    ];

    const first = chooseConflictWinner(new Map(), conflicts[0], 'E:/Mods/ModA');
    const second = chooseConflictWinner(first, conflicts[1], 'E:/Mods/ModC');
    const summary = summarizeConflictResolution(conflicts, second);

    expect(summary.disablePaths).toEqual(['E:/Mods/ModB']);
    expect(summary.resolvedCount).toBe(2);
    expect(summary.unresolvedCount).toBe(0);
  });

  it('marks an earlier group unresolved when a later choice keeps its disabled participant', () => {
    const conflicts = [
      conflict('aaaaaaaa', ['E:/Mods/ModA', 'E:/Mods/ModB']),
      conflict('bbbbbbbb', ['E:/Mods/ModB', 'E:/Mods/ModC']),
    ];
    const first = chooseConflictWinner(new Map(), conflicts[0], 'E:/Mods/ModA');
    const contradictory = chooseConflictWinner(first, conflicts[1], 'E:/Mods/ModB');

    expect(summarizeConflictResolution(conflicts, contradictory)).toMatchObject({
      disablePaths: ['E:/Mods/ModC'],
      resolvedCount: 1,
      unresolvedCount: 1,
    });
  });

  it('deduplicates disabled paths and ignores stale decisions outside current conflicts', () => {
    const conflicts = [
      conflict('aaaaaaaa', ['E:/Mods/ModA', 'E:/Mods/ModB']),
      conflict('bbbbbbbb', ['E:/Mods/ModB', 'E:/Mods/ModC']),
    ];
    let decisions = setModDecision(new Map(), 'E:/Mods/ModB', 'disable');
    decisions = setModDecision(decisions, 'E:/Mods/Deleted', 'disable');

    expect(summarizeConflictResolution(conflicts, decisions).disablePaths).toEqual([
      'E:/Mods/ModB',
    ]);
  });
});
