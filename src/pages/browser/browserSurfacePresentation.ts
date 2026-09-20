export const MIN_LIVE_BROWSER_WIDTH_PX = 560;
export const DOWNLOAD_PANEL_WIDTH_PX = 400;
export const LIBRARY_PANEL_WIDTH_PX = 512;
export const APP_MENU_BROWSER_LEFT_INSET_PX = 240;

export type BrowserSidePanel = 'downloads' | 'library' | null;
export type BrowserSidePanelLayout = 'none' | 'docked' | 'full';

export interface BrowserSurfacePresentation {
  visibility: 'visible' | 'hidden';
  focus: 'browser' | 'dom';
  leftInsetPx: number;
}

interface BrowserSurfacePresentationInput {
  activeSidePanel: BrowserSidePanel;
  availableWidth: number;
  hasBlockingOverlay: boolean;
  isAppMenuOpen: boolean;
  isChromeTrayOpen: boolean;
}

function getSidePanelWidth(panel: Exclude<BrowserSidePanel, null>): number {
  return panel === 'downloads' ? DOWNLOAD_PANEL_WIDTH_PX : LIBRARY_PANEL_WIDTH_PX;
}

export function getBrowserSidePanelLayout(
  activeSidePanel: BrowserSidePanel,
  availableWidth: number,
): BrowserSidePanelLayout {
  if (!activeSidePanel) return 'none';

  return availableWidth >= MIN_LIVE_BROWSER_WIDTH_PX + getSidePanelWidth(activeSidePanel)
    ? 'docked'
    : 'full';
}

export function getBrowserSurfacePresentation({
  activeSidePanel,
  availableWidth,
  hasBlockingOverlay,
  isAppMenuOpen,
  isChromeTrayOpen,
}: BrowserSurfacePresentationInput): BrowserSurfacePresentation {
  if (hasBlockingOverlay || getBrowserSidePanelLayout(activeSidePanel, availableWidth) === 'full') {
    return { visibility: 'hidden', focus: 'dom', leftInsetPx: 0 };
  }

  const hasDomControl = Boolean(activeSidePanel) || isAppMenuOpen || isChromeTrayOpen;

  return {
    visibility: 'visible',
    focus: hasDomControl ? 'dom' : 'browser',
    leftInsetPx: isAppMenuOpen ? APP_MENU_BROWSER_LEFT_INSET_PX : 0,
  };
}
