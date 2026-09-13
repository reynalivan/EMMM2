import { describe, expect, it } from 'vitest';
import { toModHealthPanelData } from './modHealthPresentation';

describe('toModHealthPanelData', () => {
  it('maps backend support, manifest counts, and shape variables for the preview card', () => {
    const report = toModHealthPanelData({
      support_level: 'experimental',
      issues: [
        {
          severity: 'warning',
          code: 'orphan_asset',
          message: 'unused.buf is not referenced.',
          file_path: 'unused.buf',
          section: null,
          line: null,
        },
      ],
      manifest: {
        referenced: [],
        inactive_only: [],
        orphan: [],
        external_reference: [],
        counts: { referenced: 3, inactive_only: 2, orphan: 1, external_reference: 0 },
      },
      file_manifest: [],
      controls: [
        {
          kind: 'shape_variable',
          section: 'ShapeBody',
          file_path: 'mod.ini',
          key: null,
          back: null,
          variable: 'body_scale',
          values: ['0.8', '1.0'],
          default_value: '1.0',
        },
      ],
    });

    expect(report).toEqual({
      supportLevel: 'experimental',
      issues: [
        {
          severity: 'warning',
          code: 'orphan_asset',
          message: 'unused.buf is not referenced.',
          filePath: 'unused.buf',
          line: null,
        },
      ],
      assets: { referenced: 3, inactiveOnly: 2, orphan: 1, externalReference: 0 },
      assetEntries: [],
      controls: [
        {
          kind: 'shape',
          name: 'body_scale',
          key: null,
          back: null,
          values: ['0.8', '1.0'],
          defaultValue: '1.0',
        },
      ],
    });
  });

  it('maps basic support to the baseline UI state', () => {
    expect(
      toModHealthPanelData({
        support_level: 'basic',
        issues: [],
        manifest: {
          referenced: [],
          inactive_only: [],
          orphan: [],
          external_reference: [],
          counts: { referenced: 0, inactive_only: 0, orphan: 0, external_reference: 0 },
        },
        file_manifest: [],
        controls: [],
      }).supportLevel,
    ).toBe('baseline');
  });
});
