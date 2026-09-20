import { describe, expect, it } from 'vitest';
import {
  APP_MENU_BROWSER_LEFT_INSET_PX,
  getBrowserSidePanelLayout,
  getBrowserSurfacePresentation,
} from './browserSurfacePresentation';

describe('browser surface presentation', () => {
  it('keeps the browser visible and focused when no browser UI is open', () => {
    expect(
      getBrowserSurfacePresentation({
        activeSidePanel: null,
        availableWidth: 1200,
        hasBlockingOverlay: false,
        isAppMenuOpen: false,
        isChromeTrayOpen: false,
      }),
    ).toEqual({ visibility: 'visible', focus: 'browser', leftInsetPx: 0 });
  });

  it('keeps the browser visible but hands DOM controls focus for non-blocking UI', () => {
    expect(
      getBrowserSurfacePresentation({
        activeSidePanel: 'downloads',
        availableWidth: 1200,
        hasBlockingOverlay: false,
        isAppMenuOpen: true,
        isChromeTrayOpen: true,
      }),
    ).toEqual({
      visibility: 'visible',
      focus: 'dom',
      leftInsetPx: APP_MENU_BROWSER_LEFT_INSET_PX,
    });
  });

  it('hides the browser for blocking UI regardless of other browser UI', () => {
    expect(
      getBrowserSurfacePresentation({
        activeSidePanel: 'downloads',
        availableWidth: 1200,
        hasBlockingOverlay: true,
        isAppMenuOpen: true,
        isChromeTrayOpen: true,
      }),
    ).toEqual({ visibility: 'hidden', focus: 'dom', leftInsetPx: 0 });
  });

  it('uses a full panel when the remaining browser viewport would be too narrow', () => {
    expect(getBrowserSidePanelLayout('downloads', 959)).toBe('full');
    expect(getBrowserSidePanelLayout('downloads', 960)).toBe('docked');
    expect(getBrowserSidePanelLayout('library', 1071)).toBe('full');
    expect(getBrowserSidePanelLayout('library', 1072)).toBe('docked');
  });

  it('hides the browser while a side panel occupies the full viewport', () => {
    expect(
      getBrowserSurfacePresentation({
        activeSidePanel: 'library',
        availableWidth: 700,
        hasBlockingOverlay: false,
        isAppMenuOpen: false,
        isChromeTrayOpen: false,
      }),
    ).toEqual({ visibility: 'hidden', focus: 'dom', leftInsetPx: 0 });
  });
});
