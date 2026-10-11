import { describe, expect, it } from 'vitest';
import type { KeyBindSectionGroup } from '../utils/previewPanelUtils';
import { mergeIniEditorSnapshot, type IniEditorBaseline } from './iniEditorSnapshot';

function group(fileName: string, value: string, sourceHash: string): KeyBindSectionGroup {
  return {
    id: fileName,
    fileName,
    rangeLabel: 'line 1',
    sections: [
      {
        sectionName: 'KeyMain',
        fields: [
          {
            id: `${fileName}:KeyMain:key:1`,
            fileName,
            sectionName: 'KeyMain',
            lineIdx: 1,
            label: 'key',
            prefix: 'key =',
            value,
            sourceHash,
          },
        ],
      },
    ],
  };
}

describe('mergeIniEditorSnapshot', () => {
  it('merges only confirmed files and keeps other file snapshots frozen', () => {
    const alpha = group('alpha.ini', 'A', 'alpha-old');
    const beta = group('beta.ini', 'B', 'beta-old');
    const previous: IniEditorBaseline = {
      sessionKey: 'game\0path',
      sections: [alpha, beta],
      values: { 'alpha.ini:KeyMain:key:1': 'A', 'beta.ini:KeyMain:key:1': 'B' },
      rawLinesByFile: {
        'alpha.ini': ['[KeyMain]', 'key = A'],
        'beta.ini': ['[KeyMain]', 'key = B'],
      },
    };
    const merged = mergeIniEditorSnapshot(
      previous,
      {
        sections: [
          group('alpha.ini', 'A2', 'alpha-new'),
          group('beta.ini', 'EXTERNAL', 'beta-new'),
        ],
        rawLinesByFile: {
          'alpha.ini': ['[KeyMain]', 'key = A2'],
          'beta.ini': ['[KeyMain]', 'key = EXTERNAL'],
        },
        readVersion: 2,
      },
      new Set(['alpha.ini']),
    );

    expect(merged.sections).toHaveLength(2);
    expect(
      merged.sections.find((entry) => entry.fileName === 'alpha.ini')?.sections[0].fields[0],
    ).toMatchObject({
      value: 'A2',
      sourceHash: 'alpha-new',
    });
    expect(merged.sections.find((entry) => entry.fileName === 'beta.ini')).toEqual(beta);
    expect(merged.values).toEqual({
      'alpha.ini:KeyMain:key:1': 'A2',
      'beta.ini:KeyMain:key:1': 'B',
    });
    expect(merged.rawLinesByFile['beta.ini']).toEqual(previous.rawLinesByFile['beta.ini']);
  });
});
