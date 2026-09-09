import { describe, expect, it } from 'vitest';
import { getPreviousExplorerSubPath } from './useFolderGridRuntime';

describe('getPreviousExplorerSubPath', () => {
  it('targets the direct parent view for a breadcrumb folder switcher', () => {
    expect(getPreviousExplorerSubPath('SkinSelectImpact/Aglaea')).toBe('SkinSelectImpact');
    expect(getPreviousExplorerSubPath('E:/Mods/SkinSelectImpact/Aglaea')).toBe(
      'E:/Mods/SkinSelectImpact',
    );
    expect(getPreviousExplorerSubPath('SkinSelectImpact')).toBeUndefined();
  });
});
