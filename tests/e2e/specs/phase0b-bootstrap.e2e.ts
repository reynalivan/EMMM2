import { expect } from '@wdio/globals';
import { createMockGame, removeMockGame, type MockGame } from '../support/fixtures.js';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { invokeInApp } from '../support/ipc.js';

/**
 * Fase 0b — Bootstrap & startup checks (tc-01 edges). Config-status branching
 * and the recovery-task check. Crash-injected recovery resume stays
 * [manual-smoke]; here we assert the clean-boot contracts.
 */
describe('Fase 0b — Bootstrap & Startup', () => {
  let game: MockGame;

  before(async () => {
    game = await createMockGame('Phase0b');
    await seedGameAndOpenDashboard(game);
  });

  after(async () => {
    await removeMockGame(game);
  });

  it('TC-01-03: Config status reports configured after a game exists', async () => {
    const status = await invokeInApp('check_config_status');
    expect(status).toBeDefined();
    // App routes to dashboard only when this equals 'HasConfig'.
    expect(status).toBe('HasConfig');
  });

  it('TC-01-04: Startup recovery check returns a task list (empty on clean boot)', async () => {
    const tasks = await invokeInApp<unknown[]>('app_startup_check');
    expect(Array.isArray(tasks)).toBe(true);
  });
});
