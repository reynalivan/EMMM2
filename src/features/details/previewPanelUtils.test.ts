import { describe, expect, it } from 'vitest';
import {
  buildKeyBindSections,
  buildVariableInfoSummaries,
  shouldLoadGalleryImage,
  toFieldValueMap,
  toIniWritePayload,
  validateIniDraftValue,
} from './previewPanelUtils';

describe('previewPanelUtils', () => {
  it('loads current and adjacent gallery images including wrap-around', () => {
    expect(shouldLoadGalleryImage(0, 0, 5)).toBe(true);
    expect(shouldLoadGalleryImage(1, 0, 5)).toBe(true);
    expect(shouldLoadGalleryImage(4, 0, 5)).toBe(true);
    expect(shouldLoadGalleryImage(3, 0, 5)).toBe(false);
  });

  it('validates ini draft values', () => {
    expect(validateIniDraftValue('')).toBe('Value cannot be empty.');
    expect(validateIniDraftValue('line1\nline2')).toBe('Value must be a single line.');
    expect(validateIniDraftValue('1')).toBeNull();
  });

  it('builds key bind sections with editable key/back and assignments', () => {
    const sections = buildKeyBindSections([
      {
        fileName: 'config.ini',
        document: {
          mode: 'Structured',
          raw_lines: ['[KeySwap]', 'key = v', 'back = b', '$active = 1', '$swapvar = 2'],
          variables: [],
          key_bindings: [
            {
              section_name: 'KeySwap',
              key: 'v',
              back: 'b',
              key_line_idx: 1,
              back_line_idx: 2,
            },
          ],
        },
      },
    ]);

    expect(sections).toHaveLength(1);
    expect(sections[0].sectionName).toBe('KeySwap');
    expect(sections[0].fields.map((field) => field.label)).toEqual([
      'key',
      'back',
      '$active',
      '$swapvar',
    ]);
  });

  it('creates write payload grouped by file and reports field errors', () => {
    const fields = [
      {
        id: 'config.ini:KeySwap:key:1',
        fileName: 'config.ini',
        sectionName: 'KeySwap',
        lineIdx: 1,
        label: 'key',
        prefix: 'key =',
        value: 'v',
      },
      {
        id: 'config.ini:KeySwap:assign:3',
        fileName: 'config.ini',
        sectionName: 'KeySwap',
        lineIdx: 3,
        label: '$active',
        prefix: '$active =',
        value: '1',
      },
    ];

    const initial = toFieldValueMap(fields);
    const draft = {
      ...initial,
      'config.ini:KeySwap:key:1': '',
      'config.ini:KeySwap:assign:3': '2',
    };

    const payloadEmpty = toIniWritePayload(fields, draft, initial);

    expect(payloadEmpty.fieldErrors).toEqual({
      'config.ini:KeySwap:key:1': 'Value cannot be empty.',
    });
    expect(payloadEmpty.updatesByFile).toEqual({
      'config.ini': [{ line_idx: 3, content: '$active = 2' }],
    });

    const draftWithInvalidKey = {
      ...initial,
      'config.ini:KeySwap:key:1': 'INVALID_KEY',
    };

    const payloadInvalidKey = toIniWritePayload(fields, draftWithInvalidKey, initial);
    expect(payloadInvalidKey.fieldErrors).toEqual({
      'config.ini:KeySwap:key:1': 'Invalid key token: "INVALID_KEY".',
    });
  });

  it('builds variable summaries with ranges and occurrences', () => {
    const summaries = buildVariableInfoSummaries([
      {
        fileName: 'a.ini',
        document: {
          mode: 'Structured',
          raw_lines: ['[KeySwap]', '$active = 1', '$swapvar = 0', '$swapvar = 3'],
          variables: [
            { name: '$active', value: '1', line_idx: 1 },
            { name: '$swapvar', value: '0', line_idx: 2 },
            { name: '$swapvar', value: '3', line_idx: 3 },
          ],
          key_bindings: [],
        },
      },
    ]);

    const swapvar = summaries.find((item) => item.name === '$swapvar');
    expect(swapvar?.minValue).toBe(0);
    expect(swapvar?.maxValue).toBe(3);
    expect(swapvar?.count).toBe(2);
  });
});
