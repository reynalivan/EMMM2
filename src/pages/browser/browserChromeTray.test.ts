import { describe, expect, it } from 'vitest';
import { getChromeTrayAnchorOffset } from './browserChromeTray';

describe('getChromeTrayAnchorOffset', () => {
  it('keeps a tab context menu aligned with the click when there is room', () => {
    expect(
      getChromeTrayAnchorOffset({
        clientX: 240,
        surfaceLeft: 0,
        surfaceWidth: 1600,
      }),
    ).toBe(240);
  });

  it('keeps the menu inside the left and right edges of the browser surface', () => {
    expect(
      getChromeTrayAnchorOffset({
        clientX: 0,
        surfaceLeft: 0,
        surfaceWidth: 1600,
      }),
    ).toBe(8);
    expect(
      getChromeTrayAnchorOffset({
        clientX: 1590,
        surfaceLeft: 0,
        surfaceWidth: 1600,
      }),
    ).toBe(1200);
  });

  it('uses the left edge when the surface is narrower than the menu', () => {
    expect(
      getChromeTrayAnchorOffset({
        clientX: 120,
        surfaceLeft: 0,
        surfaceWidth: 320,
      }),
    ).toBe(8);
  });
});
