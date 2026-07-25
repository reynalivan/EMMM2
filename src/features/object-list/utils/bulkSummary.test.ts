import { describe, expect, it } from 'vitest';
import { othersSuffix, parseTagList, resolveObjectNames, truncateNameList } from './bulkSummary';

const objects = [
  { id: 'a', name: 'Ayaka' },
  { id: 'b', name: 'Yelan' },
];

describe('resolveObjectNames', () => {
  it('maps ids to names in selection order', () => {
    expect(resolveObjectNames(new Set(['a', 'b']), objects)).toEqual(['Ayaka', 'Yelan']);
  });

  it('falls back to the raw id for unknown objects', () => {
    expect(resolveObjectNames(['ghost'], objects)).toEqual(['ghost']);
  });
});

describe('truncateNameList', () => {
  it('lists everything up to four names', () => {
    expect(truncateNameList(['a', 'b', 'c', 'd'], othersSuffix)).toBe('a, b, c, d');
  });

  it('truncates longer selections with the caller-supplied tail', () => {
    expect(truncateNameList(['a', 'b', 'c', 'd', 'e', 'f'], othersSuffix)).toBe(
      'a, b, c, d + 2 others',
    );
    expect(truncateNameList(['a', 'b', 'c', 'd', 'e'], (extra) => `and ${extra} more`)).toBe(
      'a, b, c, d and 1 more',
    );
  });

  it('handles an empty selection', () => {
    expect(truncateNameList([], othersSuffix)).toBe('');
  });
});

describe('parseTagList', () => {
  it('parses a JSON array string', () => {
    expect(parseTagList('["nsfw","wip"]')).toEqual(['nsfw', 'wip']);
  });

  it('treats empty or malformed data as no tags', () => {
    expect(parseTagList(null)).toEqual([]);
    expect(parseTagList('')).toEqual([]);
    expect(parseTagList('not json')).toEqual([]);
  });
});
