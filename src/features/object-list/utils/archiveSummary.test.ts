import { describe, expect, it } from 'vitest';
import type { ArchiveAnalysis } from '../../../types/scanner';
import {
  buildArchiveInfo,
  buildExtractionSummary,
  buildMismatchWarning,
  fileNameOf,
} from './archiveSummary';

describe('fileNameOf', () => {
  it('takes the last segment for both separator styles', () => {
    expect(fileNameOf('C:\\Mods\\Ayaka.zip')).toBe('Ayaka.zip');
    expect(fileNameOf('/mods/Ayaka.zip')).toBe('Ayaka.zip');
    expect(fileNameOf('Ayaka.zip')).toBe('Ayaka.zip');
  });
});

describe('buildExtractionSummary', () => {
  it('stays silent when nothing failed', () => {
    expect(
      buildExtractionSummary([
        { path: 'a.zip', status: 'done' },
        { path: 'b.zip', status: 'skipped' },
      ]),
    ).toBeNull();
  });

  it('lists counts and failed file names', () => {
    const summary = buildExtractionSummary([
      { path: 'C:\\Mods\\a.zip', status: 'done' },
      { path: 'C:\\Mods\\b.zip', status: 'failed' },
      { path: 'C:\\Mods\\c.zip', status: 'skipped' },
    ]);

    expect(summary).toBe('1 extracted, 1 failed, 1 skipped\nFailed: b.zip');
  });

  it('omits zero-count segments', () => {
    expect(buildExtractionSummary([{ path: 'b.zip', status: 'failed' }])).toBe(
      '1 failed\nFailed: b.zip',
    );
  });
});

describe('buildMismatchWarning', () => {
  it('returns null when everything matched', () => {
    expect(
      buildMismatchWarning([{ folder: 'x', isMatch: true, matchScorePct: 99 }], 'Ayaka'),
    ).toBeNull();
  });

  it('counts mismatches and details the first one', () => {
    const warning = buildMismatchWarning(
      [
        { folder: 'C:\\Mods\\one', isMatch: true, matchScorePct: 98 },
        { folder: 'C:\\Mods\\two', isMatch: false, matchedName: 'Yelan', matchScorePct: 41 },
        { folder: 'C:\\Mods\\three', isMatch: false, matchedName: null, matchScorePct: 12 },
      ],
      'Ayaka',
    );

    expect(warning).not.toBeNull();
    expect(warning!.message).toBe(
      '2 of 3 archive(s) may not match Ayaka\n→ two: Best match is Yelan (41%)',
    );
    expect(warning!.mismatchedPaths).toEqual(['C:\\Mods\\two', 'C:\\Mods\\three']);
  });

  it('lets the caller name what was imported', () => {
    const warning = buildMismatchWarning(
      [{ folder: 'C:\\Mods\\two', isMatch: false, matchedName: 'Yelan', matchScorePct: 41 }],
      'Ayaka',
      'dropped folder(s)',
    );

    expect(warning!.message).toBe(
      '1 of 1 dropped folder(s) may not match Ayaka\n→ two: Best match is Yelan (41%)',
    );
  });
});

describe('buildArchiveInfo', () => {
  it('maps a successful analysis onto the modal row', () => {
    const analysis = {
      file_size_bytes: 2048,
      has_ini: true,
      file_count: 7,
      is_encrypted: true,
      contains_nested_archives: false,
      entries: [],
    } as unknown as ArchiveAnalysis;

    expect(buildArchiveInfo('C:\\Mods\\Ayaka.zip', 'Ayaka.zip', analysis)).toEqual({
      path: 'C:\\Mods\\Ayaka.zip',
      name: 'Ayaka.zip',
      extension: 'zip',
      size_bytes: 2048,
      has_ini: true,
      file_count: 7,
      is_encrypted: true,
      contains_nested_archives: false,
      entries: [],
    });
  });

  it('defaults optional analysis fields', () => {
    const analysis = {
      file_size_bytes: null,
      has_ini: false,
      file_count: null,
      is_encrypted: null,
      contains_nested_archives: null,
      entries: undefined,
    } as unknown as ArchiveAnalysis;

    expect(buildArchiveInfo('a.7z', 'a.7z', analysis)).toMatchObject({
      size_bytes: 0,
      file_count: 1,
      is_encrypted: false,
      contains_nested_archives: false,
    });
  });

  it('still lists the archive when the probe failed', () => {
    expect(buildArchiveInfo('C:\\Mods\\broken.rar', 'broken.rar', null)).toEqual({
      path: 'C:\\Mods\\broken.rar',
      name: 'broken.rar',
      extension: 'rar',
      size_bytes: 0,
      has_ini: false,
      file_count: 0,
      is_encrypted: false,
      contains_nested_archives: false,
      entries: [],
    });
  });
});
