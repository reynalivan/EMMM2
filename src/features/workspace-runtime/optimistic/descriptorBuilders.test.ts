import { describe, expect, it } from 'vitest';
import {
  buildRuntimeMutationDescriptor,
  buildWorkspacePathRewritesDescriptor,
} from './descriptorBuilders';

describe('buildRuntimeMutationDescriptor', () => {
  it('includes dashboard and active keybindings scopes for folder switch mutations', () => {
    const descriptor = buildRuntimeMutationDescriptor('folderSwitch');

    expect(descriptor.refreshEvents).toContain('dashboardChanged');
    expect(descriptor.refreshEvents).toContain('activeKeybindingsChanged');
    expect(descriptor.refreshEvents).toContain('previewChanged');
    expect(descriptor.refreshEvents).toContain('conflictsChanged');
    expect(descriptor.refreshEvents).toContain('collectionsChanged');
  });

  it('deduplicates merged events when combining mutation classes', () => {
    const descriptor = buildRuntimeMutationDescriptor([
      'workspaceCorridor',
      'dashboardKeybindings',
      'workspaceCorridor',
    ]);

    expect(descriptor.refreshEvents).toEqual([
      'workspaceChanged',
      'corridorChanged',
      'dashboardChanged',
      'activeKeybindingsChanged',
    ]);
  });
});

describe('buildWorkspacePathRewritesDescriptor', () => {
  it('never drops thumbnails: a toggle keeps the identity the cache is keyed by', () => {
    const descriptor = buildWorkspacePathRewritesDescriptor(
      [{ old_path: 'E:/Mods/Alice', new_path: 'E:/Mods/DISABLED Alice' }],
      [],
    );

    expect(descriptor.rewrites).toEqual([
      { oldPath: 'E:/Mods/Alice', newPath: 'E:/Mods/DISABLED Alice' },
    ]);
    expect(descriptor.thumbnailPaths).toEqual([]);
    expect(descriptor.removedQueryKeys).toEqual([]);
  });
});
