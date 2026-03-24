import { describe, expect, it } from 'vitest';
import type { ModFolder } from '../../types/mod';
import { resolvePreviewTargetPath } from './previewTargetResolver';

const createFolder = (path: string, node_type: string): ModFolder =>
  ({
    node_type,
    classification_reasons: [],
    name: path,
    folder_name: path,
    path,
    is_enabled: true,
    is_directory: true,
    thumbnail_path: null,
    modified_at: 0,
    size_bytes: 0,
    has_info_json: false,
    is_favorite: false,
    is_misplaced: false,
    is_safe: true,
    metadata: null,
    category: null,
    warnings: [],
  }) as ModFolder;

describe('resolvePreviewTargetPath', () => {
  it('falls back to self mod path when nothing is selected', () => {
    const resolved = resolvePreviewTargetPath(null, 'E:/Mods/ParentMod', []);
    expect(resolved).toBe('E:/Mods/ParentMod');
  });

  it('anchors InternalAssets child selection to self mod path', () => {
    const resolved = resolvePreviewTargetPath('E:/Mods/ParentMod/Assets', 'E:/Mods/ParentMod', [
      createFolder('E:/Mods/ParentMod/Assets', 'InternalAssets'),
    ]);

    expect(resolved).toBe('E:/Mods/ParentMod');
  });

  it('keeps ContainerFolder selection for navigable children', () => {
    const resolved = resolvePreviewTargetPath('E:/Mods/ParentMod/Variants', 'E:/Mods/ParentMod', [
      createFolder('E:/Mods/ParentMod/Variants', 'ContainerFolder'),
    ]);

    expect(resolved).toBe('E:/Mods/ParentMod/Variants');
  });

  it('matches child path with slash and case normalization', () => {
    const resolved = resolvePreviewTargetPath(
      'e:\\mods\\parentmod\\assets\\',
      'E:/Mods/ParentMod',
      [createFolder('E:/Mods/ParentMod/Assets', 'InternalAssets')],
    );

    expect(resolved).toBe('E:/Mods/ParentMod');
  });

  it('matches unicode child path with ASCII-only case folding', () => {
    const resolved = resolvePreviewTargetPath(
      'e:\\mods\\한국character\\日本語mod\\assets\\',
      'E:/Mods/한국Character/日本語MOD',
      [createFolder('E:/Mods/한국Character/日本語MOD/Assets', 'InternalAssets')],
    );

    expect(resolved).toBe('E:/Mods/한국Character/日本語MOD');
  });

  it('falls back to self mod path for unknown nested selection', () => {
    const resolved = resolvePreviewTargetPath(
      'E:/Mods/ParentMod/UnknownChild',
      'E:/Mods/ParentMod',
      [createFolder('E:/Mods/ParentMod/KnownChild', 'ContainerFolder')],
    );

    expect(resolved).toBe('E:/Mods/ParentMod');
  });
});
