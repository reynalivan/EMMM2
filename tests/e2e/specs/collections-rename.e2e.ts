import { $, browser, expect } from '@wdio/globals';
import fs from 'node:fs/promises';
import path from 'node:path';
import type {
  ApplyResult,
  CollectionPreview,
  CollectionSummary,
} from '../../../src/shared/api/tauri/bindings.gen.js';
import { gotoWorkspaceView, seedGameAndOpenDashboard } from '../support/app.js';
import { createObject, reconcile, waitForWorkspaceCoreReady } from '../support/data.js';
import {
  addMockMod,
  createMockGame,
  scheduleMockGameRemoval,
  type MockGame,
} from '../support/fixtures.js';
import { invokeInApp } from '../support/ipc.js';

describe('Collections retain renamed saved roots', () => {
  let game: MockGame;
  let gameId: string;

  before(async () => {
    game = await createMockGame('CollectionRename');
    scheduleMockGameRemoval(game);
    gameId = await seedGameAndOpenDashboard(game);
  });

  async function save(name: string): Promise<CollectionSummary> {
    return invokeInApp<CollectionSummary>('create_collection', {
      gameId,
      name,
      saveMode: 'save_current_state',
    });
  }

  async function verifyAndApply(
    collection: CollectionSummary,
    relativePath: string,
  ): Promise<void> {
    const preview = await invokeInApp<CollectionPreview>('get_collection_preview', {
      gameId,
      collectionId: collection.id,
    });
    const root = preview.projected_state.active_roots.find(
      (item) => item.source_path.replaceAll('\\', '/') === relativePath,
    );
    expect(root).toBeDefined();
    expect(root?.is_missing).toBe(false);

    // Fixture mutations use IPC directly, so drop queries from the prior case.
    await browser.url('http://tauri.localhost/');
    await $('[data-testid="dashboard-layout"]').waitForDisplayed({ timeout: 25_000 });
    await waitForWorkspaceCoreReady(gameId);
    await gotoWorkspaceView('collections');
    const row = await $(`tr*=${collection.name}`);
    await row.waitForDisplayed();
    await row.click();
    await browser.waitUntil(
      async () => (await $('body').getText()).includes(path.basename(relativePath)),
      {
        timeout: 10_000,
        timeoutMsg: 'Collection preview did not display the renamed root',
      },
    );

    const absolutePath = path.join(game.modsPath, relativePath);
    await invokeInApp('bulk_toggle_mods', {
      gameId,
      paths: [absolutePath],
      enable: false,
      operationId: `collection-rename-disable-${collection.id}`,
      intentRevision: 1,
      expectedIdentities: null,
    });
    await waitForWorkspaceCoreReady(gameId);
    const applied = await invokeInApp<ApplyResult>('apply_collection', {
      gameId,
      collectionId: collection.id,
      ignoreMissing: false,
    });
    expect(applied.partial_apply).toBe(false);
    expect(applied.skipped_missing_paths).toEqual([]);
    expect(applied.mods_enabled).toBeGreaterThanOrEqual(1);
    expect((await fs.stat(absolutePath)).isDirectory()).toBe(true);
    await invokeInApp('delete_collection', { id: collection.id });
  }

  it('applies a live snapshot after a mod rename through native IPC', async () => {
    await createObject(gameId, 'NativeModRename');
    const oldPath = await addMockMod(game, 'NativeModRename', 'BeforeRename');
    await reconcile(gameId);
    const collection = await save('Native saved mod rename');
    await invokeInApp('rename_mod_folder', {
      gameId,
      folderPath: oldPath,
      newName: 'AfterRename',
    });
    await reconcile(gameId);
    await verifyAndApply(collection, 'NativeModRename/AfterRename');
  });

  it('applies a live snapshot after an object identity rename on disk', async () => {
    await createObject(gameId, 'NativeObjectBefore');
    await addMockMod(game, 'NativeObjectBefore', 'ObjectSkin');
    await reconcile(gameId);
    const collection = await save('Native saved object rename');
    await fs.rename(
      path.join(game.modsPath, 'NativeObjectBefore'),
      path.join(game.modsPath, 'NativeObjectAfter'),
    );
    await reconcile(gameId);
    await verifyAndApply(collection, 'NativeObjectAfter/ObjectSkin');
  });
});
