import { $, browser, expect } from '@wdio/globals';
import fs from 'node:fs/promises';
import path from 'node:path';
import {
  addMockMod,
  addMockMods,
  createMockGame,
  scheduleMockGameRemoval,
  type MockGame,
} from '../support/fixtures.js';
import { gotoWorkspaceView, seedGameAndOpenDashboard } from '../support/app.js';
import { getObjects, waitForWorkspaceCoreReady } from '../support/data.js';
import { invokeInApp } from '../support/ipc.js';
import type {
  BulkResult,
  MoveModsToObjectInput,
  WorkspaceExplorerPage,
  WorkspaceExplorerQuery,
  WorkspaceMoveTarget,
} from '../../../src/shared/api/tauri/bindings.gen.js';

const SOURCE = 'LifecycleSource';
const TARGET = 'LifecycleTarget';
const PAGE_MODS = 205;
const TIMEOUT = 30_000;

describe('Mods lifecycle hardening in the native application', function () {
  this.timeout(120_000);
  let game: MockGame;
  let gameId: string;
  let targetId: string;
  let moveSource: string;
  let outside: string;

  const query = (search: string | null = 'Page'): WorkspaceExplorerQuery => ({
    game_id: gameId,
    explorer_sub_path: SOURCE,
    search_query: search,
    sort_field: 'name',
    sort_order: 'asc',
    safety_filter: 'all',
  });
  const page = (cursor: string | null, search: string | null = 'Page') =>
    invokeInApp<WorkspaceExplorerPage>('get_workspace_explorer_page', {
      input: { query: query(search), cursor, page_size: 100 },
    });

  before(async () => {
    expect(await invokeInApp<string>('plugin:app|identifier')).toBe('com.reynalivan.emmm.e2e');
    game = await createMockGame('mods_lifecycle');
    await addMockMods(
      game,
      SOURCE,
      Array.from({ length: PAGE_MODS }, (_, index) => `Page${String(index).padStart(3, '0')}`),
    );
    moveSource = await addMockMod(game, SOURCE, 'MoveMe');
    await fs.writeFile(path.join(moveSource, 'mesh.buf'), 'owned native move asset');
    const anchor = await addMockMod(game, TARGET, 'Anchor');
    await fs.writeFile(path.join(anchor, 'mesh.buf'), 'owned native sibling asset');
    await fs.mkdir(path.join(game.modsPath, TARGET, 'Container'));
    outside = path.join(game.fixtureRoot, 'OutsideMods');
    await fs.mkdir(outside);
    await fs.symlink(outside, path.join(game.modsPath, TARGET, 'Linked'), 'junction');
    gameId = await seedGameAndOpenDashboard(game);
    const target = (await getObjects(gameId)).find((object) => object.folder_path === TARGET);
    if (!target) throw new Error('Owned destination object was not indexed');
    targetId = target.id;
    await gotoWorkspaceView('mods');
    const source = (await getObjects(gameId)).find((object) => object.folder_path === SOURCE);
    if (!source) throw new Error('Owned source object was not indexed');
    await $(`[data-object-id="${source.id}"]`).click();
    await $('[data-testid="folder-grid"]').waitForDisplayed({ timeout: TIMEOUT });
    await $('[data-testid="view-grid"]').click();
    await browser.waitUntil(
      async () => {
        const grid = await $('[data-testid="folder-grid"]');
        return (await grid.$$('[role="gridcell"]').length) > 0;
      },
      { timeout: TIMEOUT, timeoutMsg: 'Native source grid did not finish its first page' },
    );
  });

  after(() => {
    if (game) scheduleMockGameRemoval(game);
  });

  it('keeps a three-page native cursor snapshot query-bound', async () => {
    const first = await page(null);
    expect(first.total_matching).toBe(PAGE_MODS);
    expect(first.items).toHaveLength(100);
    if (!first.next_cursor) throw new Error('First page omitted its cursor');
    const second = await page(first.next_cursor);
    expect(second.listing_revision).toBe(first.listing_revision);
    expect(second.items).toHaveLength(100);
    if (!second.next_cursor) throw new Error('Second page omitted its cursor');
    const third = await page(second.next_cursor);
    expect(third.listing_revision).toBe(first.listing_revision);
    expect(third.items).toHaveLength(5);
    expect(third.next_cursor).toBeNull();
    const items = [...first.items, ...second.items, ...third.items];
    expect(new Set(items.map((item) => item.path)).size).toBe(PAGE_MODS);

    let rejected = false;
    try {
      await page(first.next_cursor, 'Different query');
    } catch {
      rejected = true;
    }
    expect(rejected).toBe(true);

    for (let index = 0; index < 8; index += 1) {
      await page(null, `EvictionQuery${index}`);
    }
    let expired = '';
    try {
      await page(first.next_cursor);
    } catch (error: unknown) {
      expired = String(error);
    }
    expect(expired).toMatch(/ExplorerSnapshotExpired|explorer_snapshot_expired/);
  });

  it('does not throw native WebView errors when navigating empty search results', async () => {
    const searchControl = await $('[data-testid="mod-grid-search"]');
    await searchControl.waitForClickable({ timeout: TIMEOUT });
    await searchControl.click();
    const search = await searchControl.$('input[type="search"]');
    await search.waitForClickable({ timeout: TIMEOUT });
    await search.setValue('NoLifecycleMatch');
    await $('p=No mods match your search.').waitForDisplayed({ timeout: TIMEOUT });
    await browser.execute(() => {
      const scope = window as Window & { __EMMM_NAV_ERRORS__?: string[] };
      scope.__EMMM_NAV_ERRORS__ = [];
      window.addEventListener('error', (event) => scope.__EMMM_NAV_ERRORS__?.push(event.message));
      const grid = document.querySelector<HTMLElement>('[data-testid="folder-grid"]');
      if (!grid) throw new Error('Native empty grid is missing');
      grid.focus();
    });
    await browser.keys(['ArrowRight', 'ArrowLeft', 'ArrowDown', 'Home']);
    const errors = await browser.execute(
      () => (window as Window & { __EMMM_NAV_ERRORS__?: string[] }).__EMMM_NAV_ERRORS__ ?? [],
    );
    expect(errors).toEqual([]);
    await search.setValue('');
  });

  it('rejects an outside-root junction without moving any asset', async () => {
    await waitForWorkspaceCoreReady(gameId);
    const targets = await invokeInApp<WorkspaceMoveTarget[]>('list_move_targets_for_object', {
      gameId,
      objectId: targetId,
    });
    expect(targets.some((target) => target.target_subpath === 'Container')).toBe(true);
    expect(targets.some((target) => target.target_subpath === 'Linked')).toBe(false);
    const input: MoveModsToObjectInput = {
      game_id: gameId,
      folder_paths: [moveSource],
      target_object_id: targetId,
      target_subpath: 'Linked',
      status: 'only-enable',
    };
    let rejected = false;
    try {
      await invokeInApp('move_mods_to_object', { input, recoveryIds: null });
    } catch {
      rejected = true;
    }
    expect(rejected).toBe(true);
    expect(await fs.readFile(path.join(moveSource, 'mesh.buf'), 'utf8')).toBe(
      'owned native move asset',
    );
    expect(await fs.readdir(outside)).toEqual([]);
  });

  it('moves exclusively into a container and survives a native restart', async () => {
    await waitForWorkspaceCoreReady(gameId);
    const input: MoveModsToObjectInput = {
      game_id: gameId,
      folder_paths: [moveSource],
      target_object_id: targetId,
      target_subpath: 'Container',
      status: 'only-enable',
    };
    const result = await invokeInApp<BulkResult>('move_mods_to_object', {
      input,
      recoveryIds: null,
    });
    expect(result.failures).toEqual([]);
    const destination = path.join(game.modsPath, TARGET, 'Container', 'MoveMe', 'mesh.buf');
    expect(await fs.readFile(destination, 'utf8')).toBe('owned native move asset');
    expect(
      await fs.readFile(path.join(game.modsPath, TARGET, 'DISABLED Anchor', 'mesh.buf'), 'utf8'),
    ).toBe('owned native sibling asset');
    await browser.reloadSession();
    await browser.url('http://tauri.localhost/');
    await $('#root').waitForExist({ timeout: TIMEOUT });
    await browser.waitUntil(
      () =>
        browser.execute(() =>
          Boolean(
            (window as Window & { __TAURI__?: { core?: { invoke?: unknown } } }).__TAURI__?.core
              ?.invoke,
          ),
        ),
      { timeout: TIMEOUT, timeoutMsg: 'Restarted native app omitted its IPC bridge' },
    );
    expect(await invokeInApp<string>('plugin:app|identifier')).toBe('com.reynalivan.emmm.e2e');
    await waitForWorkspaceCoreReady(gameId);
    expect(await fs.readFile(destination, 'utf8')).toBe('owned native move asset');
    expect(
      await fs.readFile(path.join(game.modsPath, TARGET, 'DISABLED Anchor', 'mesh.buf'), 'utf8'),
    ).toBe('owned native sibling asset');
  });

  it('applies a saved collection through the receipted completion boundary', async () => {
    await waitForWorkspaceCoreReady(gameId);
    const collection = await invokeInApp<{ id: string }>('create_collection', {
      gameId,
      name: 'Native lifecycle receipt',
      saveMode: 'save_current_state',
    });
    const modFolder = path.join(game.modsPath, TARGET, 'Container', 'MoveMe');
    await invokeInApp('bulk_toggle_mods', {
      gameId,
      paths: [modFolder],
      enable: false,
      operationId: 'native-lifecycle-disable',
      intentRevision: 1,
      expectedIdentities: null,
    });
    await waitForWorkspaceCoreReady(gameId);
    await fs.access(path.join(game.modsPath, TARGET, 'Container', 'DISABLED MoveMe', 'mesh.buf'));
    const applied = await invokeInApp<{ mods_enabled: number; partial_apply: boolean }>(
      'apply_collection',
      { collectionId: collection.id, gameId, ignoreMissing: false },
    );
    expect(applied.mods_enabled).toBe(1);
    expect(applied.partial_apply).toBe(false);
    expect(await fs.readFile(path.join(modFolder, 'mesh.buf'), 'utf8')).toBe(
      'owned native move asset',
    );
    await waitForWorkspaceCoreReady(gameId);
  });
});
