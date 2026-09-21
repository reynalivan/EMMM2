export const BROWSER_CHROME_TRAY_EDGE_GAP_PX = 8;
export const BROWSER_CHROME_TRAY_MENU_WIDTH_PX = 384;

interface ChromeTrayAnchorInput {
  clientX: number;
  surfaceLeft: number;
  surfaceWidth: number;
}

export function getChromeTrayAnchorOffset({
  clientX,
  surfaceLeft,
  surfaceWidth,
}: ChromeTrayAnchorInput): number {
  const minimum = BROWSER_CHROME_TRAY_EDGE_GAP_PX;
  const maximum = Math.max(
    minimum,
    surfaceWidth - BROWSER_CHROME_TRAY_MENU_WIDTH_PX - BROWSER_CHROME_TRAY_EDGE_GAP_PX * 2,
  );

  return Math.min(Math.max(clientX - surfaceLeft, minimum), maximum);
}
