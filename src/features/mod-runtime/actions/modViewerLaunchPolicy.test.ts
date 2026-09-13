import { describe, expect, it } from 'vitest';
import { GameType } from '@/entities/game';
import { getModViewerLaunchPolicy } from './modViewerLaunchPolicy';

describe('getModViewerLaunchPolicy', () => {
  it('hides every entry point until an executable is configured', () => {
    expect(getModViewerLaunchPolicy(null, GameType.GIMI)).toEqual({
      visible: false,
      experimental: false,
    });
  });

  it.each([GameType.GIMI, GameType.ZZMI, GameType.WWMI])(
    'supports game type %s normally',
    (gameType) => {
      expect(getModViewerLaunchPolicy('C:/Tools/mod_viewer.exe', gameType)).toEqual({
        visible: true,
        experimental: false,
      });
    },
  );

  it('marks SRMI as experimental', () => {
    expect(getModViewerLaunchPolicy('C:/Tools/mod_viewer.exe', GameType.SRMI)).toEqual({
      visible: true,
      experimental: true,
    });
  });

  it('hides EFMI and unknown games', () => {
    expect(getModViewerLaunchPolicy('C:/Tools/mod_viewer.exe', GameType.EFMI)).toEqual({
      visible: false,
      experimental: false,
    });
    expect(getModViewerLaunchPolicy('C:/Tools/mod_viewer.exe', 99)).toEqual({
      visible: false,
      experimental: false,
    });
  });
});
