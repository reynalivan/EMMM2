import { describe, expect, it } from 'vitest';
import { parseTagList, resolveObjectNames, truncateNameList } from './bulkSummary';

const others = (extra: number) => `+ ${extra} others`;

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
    expect(truncateNameList(['a', 'b', 'c', 'd'], others)).toBe('a, b, c, d');
  });

  it('truncates longer selections with the caller-supplied tail', () => {
    expect(truncateNameList(['a', 'b', 'c', 'd', 'e', 'f'], others)).toBe('a, b, c, d + 2 others');
    expect(truncateNameList(['a', 'b', 'c', 'd', 'e'], (extra) => `and ${extra} more`)).toBe(
      'a, b, c, d and 1 more',
    );
  });

  it('handles an empty selection', () => {
    expect(truncateNameList([], others)).toBe('');
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
