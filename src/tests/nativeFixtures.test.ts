import fs from 'node:fs/promises';
import { describe, expect, it } from 'vitest';
import {
  cleanupScheduledMockGames,
  createMockGame,
  scheduleMockGameRemoval,
  removeMockGame,
} from '../../tests/e2e/support/fixtures';

describe('native fixture teardown', () => {
  it('retains a scheduled fixture until the harness invokes post-shutdown cleanup', async () => {
    const game = await createMockGame('TeardownTest');
    scheduleMockGameRemoval(game);
    expect((await fs.stat(game.root)).isDirectory()).toBe(true);
    expect(await cleanupScheduledMockGames()).toBe(1);
    await expect(fs.stat(game.root)).rejects.toMatchObject({ code: 'ENOENT' });
    expect(await cleanupScheduledMockGames()).toBe(0);
  });

  it('rejects a mismatched ownership token without deleting the fixture', async () => {
    const game = await createMockGame('OwnershipTest');
    try {
      await expect(removeMockGame({ ...game, cleanupToken: 'not-the-owner' })).rejects.toThrow(
        'unregistered fixture root',
      );
      expect((await fs.stat(game.root)).isDirectory()).toBe(true);
    } finally {
      await removeMockGame(game);
    }
  });
});
