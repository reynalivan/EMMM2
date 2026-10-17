import { $, browser, expect } from '@wdio/globals';
import { execFile, spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { promisify } from 'node:util';
import fs from 'node:fs/promises';
import path from 'node:path';
import {
  addMockMod,
  createMockGame,
  scheduleMockGameRemoval,
  type MockGame,
} from '../support/fixtures.js';
import { gotoWorkspaceView, seedGameAndOpenDashboard } from '../support/app.js';
import { getObjects, waitForWorkspaceCoreReady } from '../support/data.js';
import { invokeInApp } from '../support/ipc.js';
import type { FileLockReport } from '../../../src/shared/api/tauri/bindings.gen.js';

const TIMEOUT = 20_000;
const OBJECT = 'Lock QA';
const holders = new Set<ChildProcessWithoutNullStreams>();
let holderExecutable = '';

async function hold(directory: string): Promise<ChildProcessWithoutNullStreams> {
  const child = spawn(holderExecutable, [directory], { windowsHide: true });
  holders.add(child);
  await new Promise<void>((resolve, reject) => {
    let output = '';
    let errors = '';
    const timer = setTimeout(
      () => reject(new Error(`Owned lock fixture did not start: ${errors}`)),
      TIMEOUT,
    );
    child.stdout.on('data', (data: Buffer) => {
      output += data.toString();
      if (output.includes('EMMM_OWNED_LOCK_READY')) {
        clearTimeout(timer);
        resolve();
      }
    });
    child.stderr.on('data', (data: Buffer) => {
      errors += data.toString();
    });
    child.once('error', (error) => {
      clearTimeout(timer);
      reject(error);
    });
    child.once('exit', (code) => {
      clearTimeout(timer);
      reject(new Error(`Owned lock fixture exited ${code}: ${errors}`));
    });
  });
  return child;
}

async function release(child: ChildProcessWithoutNullStreams): Promise<void> {
  if (child.exitCode === null && child.signalCode === null) {
    await new Promise<void>((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error('Owned lock holder did not exit')), TIMEOUT);
      child.once('exit', () => {
        clearTimeout(timer);
        resolve();
      });
      child.stdin.end('release\n');
    });
  }
  holders.delete(child);
}

async function renameThroughUi(name: string, destination: string): Promise<void> {
  const card = await $(
    `//*[@data-testid="folder-grid"]//*[@role="gridcell"][.//h3[@title="${name}"]]`,
  );
  await card.waitForDisplayed({ timeout: TIMEOUT });
  await card.click();
  const more = await $('button[title="More Actions"]');
  await more.waitForClickable({ timeout: TIMEOUT });
  await more.click();
  const rename = await $('//ul[@role="menu"]//button[normalize-space(.)="Rename"]');
  await rename.waitForClickable({ timeout: TIMEOUT });
  await rename.click();
  const input = await card.$('h3 input');
  await input.waitForClickable({ timeout: TIMEOUT });
  await input.click();
  await browser.keys(['Control', 'a']);
  await browser.keys(['Backspace']);
  await browser.keys(Array.from(destination));
  await browser.keys(['Enter']);
  await $('[data-testid="file-lock-dialog"]').waitForDisplayed({ timeout: TIMEOUT });
  await $('[data-testid="file-lock-retry"]').waitForClickable({ timeout: TIMEOUT });
}

describe('Mods Manager persistent Windows file lock recovery', function () {
  this.timeout(120_000);
  let game: MockGame;
  let gameId: string;

  before(async () => {
    game = await createMockGame('file_locks');
    holderExecutable = path.join(game.root, 'owned-lock-holder.exe');
    await promisify(execFile)(
      'rustc',
      [path.resolve('tests/e2e/support/hold-file-lock.rs'), '-o', holderExecutable],
      { windowsHide: true },
    );
    for (const [index, name] of [
      'Persistent',
      'Cancel',
      'Terminate',
      'Replacement',
      'BulkOld',
      'BulkOther',
      'BulkNew',
    ].entries()) {
      const mod = await addMockMod(game, OBJECT, name);
      await fs.writeFile(
        path.join(mod, 'mod.ini'),
        `[TextureOverrideOwned${index}]\nhash = ${index.toString(16).padStart(8, '0')}\n`,
      );
    }
    gameId = await seedGameAndOpenDashboard(game);
    await browser.setWindowSize(1360, 900);
    await gotoWorkspaceView('mods');
    const object = (await getObjects(gameId)).find((entry) => entry.name === OBJECT);
    if (!object) throw new Error('Owned lock object was not indexed');
    await $(`[data-object-id="${object.id}"]`).click();
    await $('[data-testid="folder-grid"]').waitForDisplayed({ timeout: TIMEOUT });
  });

  after(async () => {
    for (const child of holders) {
      if (child.exitCode === null && child.signalCode === null) await release(child);
    }
    scheduleMockGameRemoval(game);
  });

  it('keeps the overlay through repeated failures and closes after unlock plus Retry', async () => {
    const source = path.join(game.modsPath, OBJECT, 'Persistent');
    const destination = path.join(game.modsPath, OBJECT, 'Persistent Renamed');
    const holder = await hold(source);
    await renameThroughUi('Persistent', 'Persistent Renamed');
    for (let attempt = 0; attempt < 2; attempt += 1) {
      const retry = await $('[data-testid="file-lock-retry"]');
      await retry.click();
      await browser.waitUntil(async () => !(await retry.isEnabled()), { timeout: TIMEOUT });
      await retry.waitForClickable({ timeout: TIMEOUT });
      await expect($('[data-testid="file-lock-dialog"]')).toBeDisplayed();
      await fs.access(source);
    }
    await release(holder);
    await $('[data-testid="file-lock-retry"]').click();
    await $('[data-testid="file-lock-dialog"]').waitForDisplayed({
      reverse: true,
      timeout: TIMEOUT,
    });
    await fs.access(destination);
    await waitForWorkspaceCoreReady(gameId);
  });

  it('Cancel prevents a later unlock from executing the pending rename', async () => {
    const source = path.join(game.modsPath, OBJECT, 'Cancel');
    const holder = await hold(source);
    await renameThroughUi('Cancel', 'Cancel Renamed');
    await $('[data-testid="file-lock-cancel"]').click();
    await $('[data-testid="file-lock-dialog"]').waitForDisplayed({
      reverse: true,
      timeout: TIMEOUT,
    });
    await release(holder);
    await waitForWorkspaceCoreReady(gameId);
    await fs.access(source);
    const entries = await fs.readdir(path.join(game.modsPath, OBJECT));
    expect(entries).not.toContain('Cancel Renamed');
  });

  it('reports the owned file user and terminates only that explicitly confirmed process', async () => {
    const source = path.join(game.modsPath, OBJECT, 'Terminate');
    const holder = await hold(source);
    if (!holder.pid) throw new Error('Owned lock holder has no PID');
    await renameThroughUi('Terminate', 'Terminate Renamed');
    const force = await $(`[data-testid="file-lock-force-${holder.pid}"]`);
    await force.waitForClickable({ timeout: TIMEOUT });
    await force.click();
    await $('[data-testid="file-lock-confirm-terminate"]').click();
    await browser.waitUntil(() => holder.exitCode !== null || holder.signalCode !== null, {
      timeout: TIMEOUT,
    });
    holders.delete(holder);
    await $('[data-testid="file-lock-retry"]').waitForClickable({ timeout: TIMEOUT });
    await expect($('[data-testid="file-lock-dialog"]')).toBeDisplayed();
    await fs.access(source);
    await $('[data-testid="file-lock-retry"]').click();
    await $('[data-testid="file-lock-dialog"]').waitForDisplayed({
      reverse: true,
      timeout: TIMEOUT,
    });
    await fs.access(path.join(game.modsPath, OBJECT, 'Terminate Renamed'));
  });

  it('preserves a newer selection when an older locked bulk delete finishes', async () => {
    const output = path.resolve('.audit-output/fix-open-matrix-20261017/native');
    await fs.mkdir(output, { recursive: true });
    const old = path.join(game.modsPath, OBJECT, 'BulkOld');
    const other = path.join(game.modsPath, OBJECT, 'BulkOther');
    const newer = path.join(game.modsPath, OBJECT, 'BulkNew');
    const firstHolder = await hold(old);
    const secondHolder = await hold(other);
    try {
      const card = (name: string) =>
        $(`//*[@data-testid="folder-grid"]//*[@role="gridcell"][.//h3[@title="${name}"]]`);
      for (const name of ['BulkOld', 'BulkOther']) {
        const item = await card(name);
        await item.moveTo();
        const checkbox = await item.$('input[type="checkbox"]:not([aria-label])');
        await checkbox.waitForClickable({ timeout: TIMEOUT });
        await checkbox.click();
        await expect(checkbox).toBeChecked();
      }
      const more = await $('.bulk-action-bar--floating button[aria-label="More actions"]');
      await more.waitForClickable({ timeout: TIMEOUT });
      await more.click();
      const trash = await (await $('.bulk-action-bar--floating')).$('button=Move to Trash');
      await trash.click();
      const confirm = await (await $('dialog:modal')).$('button=Move All to Trash');
      await confirm.waitForClickable({ timeout: TIMEOUT });
      await expect($('dialog:modal h3')).toHaveText('Delete 2 Mods?');
      await confirm.click();
      await (await $('dialog:modal')).$('button=Cancel').click();
      await $('dialog:modal').waitForExist({ reverse: true });
      const currentCard = await card('BulkNew');
      await currentCard.moveTo();
      const newerCheckbox = await currentCard.$('input[type="checkbox"]:not([aria-label])');
      await newerCheckbox.click();
      await expect(newerCheckbox).toBeChecked();
      await browser.waitUntil(
        async () => ((await currentCard.getAttribute('class')) ?? '').includes('border-primary/60'),
        { timeout: TIMEOUT },
      );
      await browser.saveScreenshot(path.join(output, 'new-selection-pending.png'));
      expect(await trash.isEnabled()).toBe(false);
      await trash.waitForEnabled({ timeout: TIMEOUT });
      await expect(newerCheckbox).toBeChecked();
      await expect(currentCard).toHaveElementClass('border-primary/60');
      await browser.saveScreenshot(path.join(output, 'new-selection-settled.png'));
      await fs.access(old);
      await fs.access(other);
      await fs.access(newer);
    } catch (error) {
      await browser.saveScreenshot(path.join(output, 'failure.png'));
      await fs.writeFile(path.join(output, 'failure.html'), await browser.getPageSource());
      throw error;
    } finally {
      await release(firstHolder);
      await release(secondHolder);
    }
  });

  it('rejects a forged recovery context through actual IPC', async () => {
    await expect(
      invokeInApp<FileLockReport>('get_file_lock_info', { recoveryId: 'not-issued' }),
    ).rejects.toThrow();
  });
});
