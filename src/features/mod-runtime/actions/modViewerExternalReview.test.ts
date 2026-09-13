import { describe, expect, it } from 'vitest';
import {
  diffModViewerManifests,
  isModViewerSnapshotAffected,
  type ModViewerManifestFile,
} from './modViewerExternalReview';

function file(
  relative_path: string,
  size_bytes: number,
  content_hash: string,
): ModViewerManifestFile {
  return { relative_path, size_bytes, content_hash };
}

describe('diffModViewerManifests', () => {
  it('reports additions, removals, and modifications with viewer-relevant categories', () => {
    const before = [
      file('mod.ini', 10, 'old-ini'),
      file('texture.dds', 20, 'old-dds'),
      file('removed.buf', 30, 'removed'),
    ];
    const after = [
      file('mod.ini', 11, 'new-ini'),
      file('texture.dds', 20, 'old-dds'),
      file('mod.ini.2026-01-01.BAK', 10, 'backup'),
      file('.mod_viewer.json', 5, 'viewer-settings'),
    ];

    expect(diffModViewerManifests(before, after)).toEqual([
      { kind: 'modified', category: 'ini', relative_path: 'mod.ini' },
      { kind: 'added', category: 'metadata', relative_path: '.mod_viewer.json' },
      { kind: 'added', category: 'backup', relative_path: 'mod.ini.2026-01-01.BAK' },
      { kind: 'removed', category: 'other', relative_path: 'removed.buf' },
    ]);
  });

  it('returns no review entries when the manifest did not change', () => {
    const manifest = [file('mod.ini', 10, 'same')];

    expect(diffModViewerManifests(manifest, manifest)).toEqual([]);
  });
});

describe('isModViewerSnapshotAffected', () => {
  it('matches a changed root that contains the launched mod folder', () => {
    expect(isModViewerSnapshotAffected('E:/Mods/Character/Mod A', ['Character'], 'E:/Mods')).toBe(
      true,
    );
  });

  it('ignores a changed root outside the launched mod folder', () => {
    expect(
      isModViewerSnapshotAffected('E:/Mods/Character/Mod A', ['Weapon/Mod B'], 'E:/Mods'),
    ).toBe(false);
  });
});
