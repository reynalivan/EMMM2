import { describe, expect, it } from 'vitest';
import { buildDemoWorkspacePreview, buildDemoWorkspaceStructure } from './workspace';

describe('demo workspace data', () => {
  it('provides split structure and preview data', () => {
    const workspace = buildDemoWorkspaceStructure({
      selected_object_folder_path: 'Characters/Nekomata',
      explorer_sub_path: null,
    });
    const preview = buildDemoWorkspacePreview({
      game_id: 'demo-zenless',
      explorer_sub_path: null,
      selected_mod_path: 'Characters/Nekomata/Streetwear',
    });

    expect(workspace.objects).toHaveLength(80);
    expect(workspace.objects.filter((object) => object.object_type === 'Character')).toHaveLength(
      50,
    );
    expect(workspace.objects.filter((object) => object.object_type === 'Environment')).toHaveLength(
      17,
    );
    expect(workspace.objects.filter((object) => object.object_type === 'UI')).toHaveLength(13);
    expect(workspace.objects.some((object) => object.name.includes('intentionally long'))).toBe(
      true,
    );
    expect(workspace.explorer.children.map((folder) => folder.name)).toEqual([
      'Streetwear',
      'Summer Palette',
      'Midnight Runner',
      'Private Variant',
      'Private Alternate',
      'Variants',
    ]);
    expect(workspace.explorer.children.filter((folder) => !folder.is_safe)).toHaveLength(2);
    expect(preview.preview.display_title).toBe('Streetwear');
    expect(workspace.runtime.source_state.status).toBe('available');
  });
});
