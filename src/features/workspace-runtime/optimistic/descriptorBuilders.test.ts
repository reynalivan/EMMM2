import { describe, expect, it } from 'vitest';
import { buildRuntimeMutationDescriptor } from './descriptorBuilders';

describe('buildRuntimeMutationDescriptor', () => {
  it('includes dashboard and active keybindings scopes for folder switch mutations', () => {
    const descriptor = buildRuntimeMutationDescriptor('folderSwitch');

    expect(descriptor.refreshEvents).toContain('dashboardChanged');
    expect(descriptor.refreshEvents).toContain('activeKeybindingsChanged');
    expect(descriptor.refreshEvents).toContain('previewChanged');
    expect(descriptor.refreshEvents).toContain('conflictsChanged');
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
