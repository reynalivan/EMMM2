import { describe, expect, it } from 'vitest';
import { parseTagList, resolveObjectNames } from './bulkSummary';

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
