import { browser, $, $$, expect } from '@wdio/globals';
import fs from 'node:fs/promises';
import path from 'node:path';
import type {
  AppSettings,
  BrowserBookmark,
  DupScanReport,
  ImportBatch,
  ImportBatchReport,
  ModInboxSnapshot,
} from '../../../src/shared/api/tauri/bindings.gen.js';
import {
  addMockMod,
  createMockGame,
  scheduleMockGameRemoval,
  type MockGame,
} from '../support/fixtures.js';
import { gotoWorkspaceView, seedGamesAndOpenDashboard } from '../support/app.js';
import { getObjects, waitForWorkspaceCoreReady } from '../support/data.js';
import { invokeInApp } from '../support/ipc.js';

const WAIT_TIMEOUT = 30_000;
const APP_URL = 'http://tauri.localhost/';
const ARTIFACT_ROOT = path.resolve('logs/client-smoke-20261007/secondary');
const ARTIFACT_STAMP = new Date().toISOString().replace(/[^a-zA-Z0-9_-]/g, '-');
const ARCHIVE_FIXTURE = path.resolve('tests/e2e/fixtures/sample-mod.zip');
const MOD_CONTENT = '[TextureOverrideQAInbox]\nhash = 0123456789abcdef\n';
const NO_APPLY_ACTIONS = './/button[starts-with(normalize-space(.),"Apply 0 Action")]';

interface SecondaryFixture {
  game: MockGame;
  gameId: string;
  inbox: string;
}

async function click(selector: string): Promise<void> {
  const control = await $(selector);
  await control.waitForClickable({ timeout: WAIT_TIMEOUT });
  await control.click();
}

function textButton(label: string): string {
  return `.//button[normalize-space(.)="${label}"]`;
}

async function expectView(view: string): Promise<void> {
  await expect(
    await $(`[data-testid="dashboard-layout"][data-workspace-view="${view}"]`),
  ).toExist();
}

async function inboxSnapshot(fixture: SecondaryFixture): Promise<ModInboxSnapshot> {
  return invokeInApp<ModInboxSnapshot>('get_mod_inbox', { gameId: fixture.gameId });
}

function isSettingsRevisionConflict(error: unknown): error is Error {
  return (
    error instanceof Error &&
    error.message.startsWith('[IPC] save_settings failed:') &&
    error.message.includes('Settings changed since this screen was loaded.')
  );
}

function setupSettings(settings: AppSettings): AppSettings {
  if (!settings.hotkeys) throw new Error('Native fixture settings omitted hotkey configuration');
  return {
    ...settings,
    language: 'en',
    hotkeys: { ...settings.hotkeys, enabled: false },
    diagnostics: { ...settings.diagnostics, telemetry_enabled: false },
  };
}

function recordSetupRetry(operation: string, error: Error): void {
  console.log(
    `EMMM_E2E_SETUP_RETRY ${JSON.stringify({ operation, originalError: error.message, retry: 1 })}`,
  );
}

async function seedFixtureGame(game: MockGame, label: string): Promise<string> {
  const name = `QA Secondary ${label}`;
  try {
    return (await seedGamesAndOpenDashboard([{ game, name }]))[0];
  } catch (error) {
    if (!isSettingsRevisionConflict(error)) throw error;
    recordSetupRetry('seed_game.save_settings', error);
  }

  // The shared seed stopped at its first save, before creating a game. Retry
  // that save once with current revision, then complete the same seed sequence.
  const fresh = await invokeInApp<AppSettings>('get_settings');
  await invokeInApp('save_settings', { settings: setupSettings(fresh) });
  const candidate = await invokeInApp<{ id: string; name: string }>('add_game_manual', {
    gameType: 'GIMI',
    path: game.root,
  });
  await invokeInApp('save_onboarding_games', { games: [{ ...candidate, name }] });
  await invokeInApp('set_active_game', { gameId: candidate.id });
  await browser.url(APP_URL);
  await $('[data-testid="dashboard-layout"]').waitForDisplayed({ timeout: WAIT_TIMEOUT });
  await waitForWorkspaceCoreReady(candidate.id, WAIT_TIMEOUT);
  return candidate.id;
}

function inboxSetupSettings(settings: AppSettings, gameId: string, inbox: string): AppSettings {
  return {
    ...setupSettings(settings),
    games: settings.games.map((candidate) =>
      candidate.id === gameId ? { ...candidate, ready_to_move_path: inbox } : candidate,
    ),
  };
}

async function startFixture(label: string, withDuplicates = false): Promise<SecondaryFixture> {
  const game = await createMockGame(`QASecondary_${label}`);
  scheduleMockGameRemoval(game);
  if (withDuplicates) {
    await addMockMod(game, 'QACloneObject', 'QACloneA');
    await addMockMod(game, 'QACloneObject', 'QACloneB');
  }
  const gameId = await seedFixtureGame(game, label);
  const inbox = path.join(game.root, 'QAInbox');
  const settings = await invokeInApp<AppSettings>('get_settings');
  try {
    await invokeInApp('save_settings', { settings: inboxSetupSettings(settings, gameId, inbox) });
  } catch (error) {
    if (!isSettingsRevisionConflict(error)) throw error;
    recordSetupRetry('seed_inbox.save_settings', error);
    const fresh = await invokeInApp<AppSettings>('get_settings');
    await invokeInApp('save_settings', { settings: inboxSetupSettings(fresh, gameId, inbox) });
  }
  await browser.url(APP_URL);
  await $('[data-testid="dashboard-layout"]').waitForDisplayed({ timeout: WAIT_TIMEOUT });
  return { game, gameId, inbox };
}

async function copyInboxArchive(fixture: SecondaryFixture): Promise<string> {
  await fs.mkdir(fixture.inbox, { recursive: true });
  const archive = path.join(fixture.inbox, 'QAArchive.zip');
  await fs.copyFile(ARCHIVE_FIXTURE, archive);
  return archive;
}

async function waitForReview(): Promise<WebdriverIO.Element> {
  const dialog = await $('dialog[aria-labelledby="match-wizard-title"]');
  await dialog.waitForDisplayed({ timeout: WAIT_TIMEOUT });
  await dialog.$('button[aria-label="Close"]').waitForClickable({ timeout: WAIT_TIMEOUT });
  await expect(await dialog.$('#match-wizard-title')).toHaveText('Review matches');
  return dialog.getElement();
}

async function scanAndReadReport(fixture: SecondaryFixture): Promise<DupScanReport> {
  await click(textButton('Start Scan'));
  await $('table[aria-label="Duplicate Scan Results"]').waitForDisplayed({
    timeout: WAIT_TIMEOUT,
  });
  await $(textButton('Start Scan')).waitForClickable({ timeout: WAIT_TIMEOUT });
  const report = await invokeInApp<DupScanReport | null>('dup_scan_get_report', {
    gameId: fixture.gameId,
  });
  if (!report) throw new Error('Rendered duplicate table has no persisted scan report');
  expect(report.groups).toHaveLength(1);
  expect(report.groups[0].confidenceScore).toBe(100);
  expect(report.groups[0].members.map((member) => member.displayName).sort()).toEqual([
    'QACloneA',
    'QACloneB',
  ]);
  return report;
}

async function openResolution(): Promise<WebdriverIO.Element> {
  await click(textButton('Apply 1 Action'));
  const dialog = await $('[role="dialog"][aria-labelledby="resolution-modal-title"]');
  await dialog.waitForDisplayed({ timeout: WAIT_TIMEOUT });
  await expect(await dialog.$('#resolution-modal-title')).toHaveText('Confirm Resolution');
  return dialog.getElement();
}

async function openBrowserMenu(): Promise<void> {
  await click('button[aria-label="Discover browser menu"]');
  await $('[data-testid="browser-toolbar-menu"]').waitForDisplayed({ timeout: WAIT_TIMEOUT });
}

async function activeTabContext(): Promise<void> {
  const tab = await $('[role="tablist"][aria-label="Discover"] button[aria-selected="true"]');
  await tab.waitForClickable({ timeout: WAIT_TIMEOUT });
  await tab.click({ button: 'right' });
  await $('[role="menu"][aria-label="Tab menu"]').waitForDisplayed({ timeout: WAIT_TIMEOUT });
}

async function browserTabCount(): Promise<number> {
  return (await $$('[role="tablist"][aria-label="Discover"] button[role="tab"]')).length;
}

async function expectTabCount(count: number): Promise<void> {
  await browser.waitUntil(async () => (await browserTabCount()) === count, {
    timeout: WAIT_TIMEOUT,
    timeoutMsg: `Discover did not render ${count} tabs`,
  });
}

describe('Native secondary client UI smoke — generated QA fixtures', () => {
  afterEach(async function () {
    if (this.currentTest?.state !== 'failed') return;
    await fs.mkdir(ARTIFACT_ROOT, { recursive: true });
    const name = `${ARTIFACT_STAMP}-${this.currentTest.title.replace(/[^a-zA-Z0-9_-]/g, '_')}`;
    const screenshot = path.join(ARTIFACT_ROOT, `${name}.png`);
    const dom = path.join(ARTIFACT_ROOT, `${name}.html`);
    await browser.saveScreenshot(screenshot);
    await fs.writeFile(dom, await browser.getPageSource(), 'utf8');
    console.log(
      `EMMM_E2E_FAILURE ${JSON.stringify({
        scenario: this.currentTest.title,
        error: this.currentTest.err?.message,
        screenshot,
        dom,
      })}`,
    );
  });

  it('creates the missing Mod Inbox with the rendered Create Folder action', async () => {
    const fixture = await startFixture('MissingInbox');
    await gotoWorkspaceView('mod-inbox');
    await expect(await $('h2=Your Mod Inbox folder does not exist yet.')).toBeDisplayed();
    expect((await inboxSnapshot(fixture)).rootState).toBe('missing');
    await expect(await $('button[aria-label="Open Inbox"]')).toBeDisabled();
    await click(textButton('Create Folder'));
    await expect(await $('h2=Inbox is clear')).toBeDisplayed();
    expect((await fs.stat(fixture.inbox)).isDirectory()).toBe(true);
    expect((await inboxSnapshot(fixture)).rootState).toBe('ready');
    await expect(await $('input[aria-label="Select all ready entries"]')).toBeDisabled();
    await expect(await $(textButton('Review selected (0)'))).toBeDisabled();
    await click('button[role="tab"][aria-label="Processed"]');
    await expect(await $('h2=No processed imports yet')).toBeDisplayed();
    await expect(await $('input[aria-label="Select all processed sources"]')).toBeDisabled();
    await click('button[role="tab"][aria-label="Ready"]');
    await click('button[aria-label="Refresh"]');
    await expect(await $('h2=Inbox is clear')).toBeDisplayed();
  });

  it('reviews selected Ready sources, closes and resumes the same batch, then cancels safely', async () => {
    const fixture = await startFixture('ReadyInbox');
    const archive = await copyInboxArchive(fixture);
    const folder = path.join(fixture.inbox, 'QALooseMod');
    await fs.mkdir(folder);
    await fs.writeFile(path.join(folder, 'mod.ini'), MOD_CONTENT);
    await gotoWorkspaceView('mod-inbox');
    await $('input[aria-label="Select QAArchive.zip"]').waitForClickable({ timeout: WAIT_TIMEOUT });
    await click('input[aria-label="Select QAArchive.zip"]');
    await expect(await $(textButton('Review selected (1)'))).toBeEnabled();
    await click('input[aria-label="Select all ready entries"]');
    await expect(await $('input[aria-label="Select QALooseMod"]')).toBeSelected();
    await expect(await $(textButton('Review selected (2)'))).toBeEnabled();
    await click('input[aria-label="Select all ready entries"]');
    await expect(await $(textButton('Review selected (0)'))).toBeDisabled();
    await click('input[aria-label="Select QAArchive.zip"]');
    await click(textButton('Review selected (1)'));
    let review = await waitForReview();
    await expect(await review.$('input[aria-label="Select DISABLED ArchivedMod"]')).toExist();
    let snapshot = await inboxSnapshot(fixture);
    const pending = snapshot.readyEntries.find((entry) => entry.name === 'QAArchive.zip');
    expect(pending?.pendingBatchId).toBeTruthy();
    const batchId = pending?.pendingBatchId;
    if (!batchId) throw new Error('Review created no pending archive batch');
    await review.$('button[aria-label="Close"]').click();
    await review.waitForDisplayed({ reverse: true, timeout: WAIT_TIMEOUT });
    await click('button[aria-label="Refresh"]');
    await $(textButton('Resume Review')).waitForClickable({ timeout: WAIT_TIMEOUT });
    await expect(await $('input[aria-label="Select QAArchive.zip"]')).toBeDisabled();
    await click(textButton('Resume Review'));
    review = await waitForReview();
    snapshot = await inboxSnapshot(fixture);
    expect(
      snapshot.readyEntries.find((entry) => entry.name === 'QAArchive.zip')?.pendingBatchId,
    ).toBe(batchId);
    await review.$(textButton('Cancel')).click();
    await review.waitForDisplayed({ reverse: true, timeout: WAIT_TIMEOUT });
    await click('button[aria-label="Refresh"]');
    await $('input[aria-label="Select QAArchive.zip"]').waitForEnabled({ timeout: WAIT_TIMEOUT });
    expect((await invokeInApp<ImportBatch>('get_import_batch', { batchId })).status).toBe(
      'cancelled',
    );
    expect(await fs.readFile(path.join(folder, 'mod.ini'), 'utf8')).toBe(MOD_CONTENT);
    expect(await fs.readFile(archive)).toEqual(await fs.readFile(ARCHIVE_FIXTURE));
    expect((await inboxSnapshot(fixture)).processedSources).toHaveLength(0);
  });

  it('selects a retained Processed QA source, cancels deletion, opens its destination, then deletes only its source', async () => {
    const fixture = await startFixture('ProcessedInbox', true);
    await copyInboxArchive(fixture);
    const target = (await getObjects(fixture.gameId)).find(
      (object) => object.name === 'QACloneObject',
    );
    if (!target) throw new Error('Generated QA destination was not indexed');
    const ready = await inboxSnapshot(fixture);
    const entry = ready.readyEntries.find((candidate) => candidate.name === 'QAArchive.zip');
    if (!entry) throw new Error('Generated QA archive is absent from the inbox');
    const created = await invokeInApp<ImportBatch>('create_mod_inbox_batch', {
      input: { gameId: fixture.gameId, entryKeys: [entry.entryKey] },
    });
    const analyzed = await invokeInApp<ImportBatch>('analyze_import_batch', {
      batchId: created.id,
    });
    for (const item of analyzed.items) {
      if (item.status === 'awaiting_category') {
        await invokeInApp('set_import_item_classification', {
          input: { itemId: item.id, category: 'Other', subCategory: null, metadata: {} },
        });
        await invokeInApp('refresh_import_item_suggestions', { itemId: item.id });
      }
      await invokeInApp('set_import_item_decision', {
        input: {
          itemId: item.id,
          decision: 'reallocate',
          destinationObjectId: target.id,
          destinationPath: null,
          canonicalEntryKey: null,
          matchedAlias: null,
        },
      });
    }
    await invokeInApp('mark_import_batch_review_started', { batchId: created.id });
    const reviewed = await invokeInApp<ImportBatch>('get_import_batch', { batchId: created.id });
    expect(reviewed.items.every((item) => item.status === 'ready')).toBe(true);
    expect(reviewed.items.every((item) => item.analysisAckRevision === item.analysisRevision)).toBe(
      true,
    );
    const committed = await invokeInApp<ImportBatchReport>('commit_import_batch', {
      input: { batchId: created.id, itemIds: reviewed.items.map((item) => item.id) },
    });
    expect(committed.failed).toBe(0);
    const seeded = await inboxSnapshot(fixture);
    expect(seeded.processedSources).toHaveLength(1);
    const source = seeded.processedSources[0];
    if (!source.processedPath || source.destinations.length !== 1) {
      throw new Error('Processed setup did not retain one QA archive with one destination');
    }
    const sourceRelativePath = path.relative(
      path.toNamespacedPath(fixture.game.fixtureRoot),
      path.toNamespacedPath(source.processedPath),
    );
    expect(sourceRelativePath).not.toBe('');
    expect(path.isAbsolute(sourceRelativePath)).toBe(false);
    expect(sourceRelativePath.startsWith('..')).toBe(false);
    const destination = source.destinations[0];
    const destinationPath = path.resolve(fixture.game.modsPath, destination.placedPath);
    const destinationRelativePath = path.relative(
      path.toNamespacedPath(fixture.game.modsPath),
      path.toNamespacedPath(destinationPath),
    );
    expect(destinationRelativePath).not.toBe('');
    expect(path.isAbsolute(destinationRelativePath)).toBe(false);
    expect(destinationRelativePath.startsWith('..')).toBe(false);
    const originalPayload = await fs.readFile(path.join(destinationPath, 'mod.ini'));
    await gotoWorkspaceView('mod-inbox');
    await click('button[role="tab"][aria-label="Processed"]');
    await $('input[aria-label="Select QAArchive.zip"]').waitForClickable({ timeout: WAIT_TIMEOUT });
    await click('input[aria-label="Select all processed sources"]');
    await expect(await $('input[aria-label="Select QAArchive.zip"]')).toBeSelected();
    await click(textButton('Delete selected (1)'));
    let deletion = await $('dialog[aria-labelledby="mod-inbox-delete-title"]');
    await deletion.waitForDisplayed({ timeout: WAIT_TIMEOUT });
    await deletion.$(textButton('Cancel')).click();
    await deletion.waitForDisplayed({ reverse: true, timeout: WAIT_TIMEOUT });
    expect((await fs.stat(source.processedPath)).isFile()).toBe(true);
    await click('button[aria-label="Open QACloneObject in app"]');
    await expectView('mods');
    await $('[data-testid="folder-grid"]').waitForDisplayed({ timeout: WAIT_TIMEOUT });
    await expect(await $('h3[title="DISABLED ArchivedMod"]')).toExist();
    await gotoWorkspaceView('mod-inbox');
    await click('button[role="tab"][aria-label="Processed"]');
    await click('input[aria-label="Select QAArchive.zip"]');
    await click(textButton('Delete selected (1)'));
    deletion = await $('dialog[aria-labelledby="mod-inbox-delete-title"]');
    await deletion.$(textButton('Delete Source')).waitForClickable({ timeout: WAIT_TIMEOUT });
    await deletion.$(textButton('Delete Source')).click();
    await deletion.waitForDisplayed({ reverse: true, timeout: WAIT_TIMEOUT });
    await expect(await $('input[aria-label="Select QAArchive.zip"]')).toBeDisabled();
    const result = await inboxSnapshot(fixture);
    expect(result.processedSources[0].sourceDeletedAt).toBeTruthy();
    const retainedArchivePath = source.processedPath;
    await browser.waitUntil(
      async () => {
        try {
          await fs.stat(retainedArchivePath);
          return false;
        } catch (error) {
          if (error instanceof Error && 'code' in error && error.code === 'ENOENT') {
            return true;
          }
          throw error;
        }
      },
      {
        timeout: WAIT_TIMEOUT,
        timeoutMsg: `Delete Source left its owned archive on disk: ${retainedArchivePath}`,
      },
    );
    console.log(
      `EMMM_QA_RECYCLE_CANDIDATE ${JSON.stringify({
        action: 'processed_source_delete',
        originalPath: source.processedPath,
        fixtureRoot: fixture.game.fixtureRoot,
      })}`,
    );
    expect(result.processedSources[0].destinations).toEqual(source.destinations);
    expect(await fs.readFile(path.join(destinationPath, 'mod.ini'))).toEqual(originalPayload);
  });

  it('scans QA duplicates, filters confidence, cancels Keep, resolves Ignore, recovers and resolves Keep', async () => {
    const fixture = await startFixture('Duplicates', true);
    await gotoWorkspaceView('storage-optimizer');
    await expect(await $(NO_APPLY_ACTIONS)).toBeDisabled();
    const initialReport = await scanAndReadReport(fixture);
    const retainedMember = initialReport.groups[0].members.find(
      (member) => member.displayName === 'QACloneA',
    );
    if (!retainedMember) throw new Error('Native duplicate report omitted generated QACloneA');
    for (const label of ['Medium', 'Low']) {
      await click(`//button[@role="tab" and normalize-space(.)="${label}"]`);
      await expect(await $('h3=No duplicates found in this category')).toBeDisplayed();
    }
    await click('//button[@role="tab" and normalize-space(.)="High"]');
    await expect(await $('table[aria-label="Duplicate Scan Results"]')).toBeDisplayed();
    await click('//button[@role="tab" and normalize-space(.)="All"]');
    await click('[role="button"][aria-label$=": QACloneA"]');
    let resolution = await openResolution();
    await expect(resolution).toHaveText(/QACloneA/);
    await resolution.$(textButton('Cancel')).click();
    await resolution.waitForDisplayed({ reverse: true, timeout: WAIT_TIMEOUT });
    expect(
      (await fs.stat(path.join(fixture.game.modsPath, 'QACloneObject', 'QACloneB'))).isDirectory(),
    ).toBe(true);
    await expect(await $('select[aria-label="Action"]')).toHaveValue(retainedMember.folderPath);
    await $('select[aria-label="Action"]').selectByVisibleText('Ignore / Whitelist');
    resolution = await openResolution();
    await expect(resolution).toHaveText(/Whitelist all 2 members/);
    await resolution.$(textButton('Confirm & Resolve')).click();
    await resolution.waitForDisplayed({ reverse: true, timeout: WAIT_TIMEOUT });
    await $('button[title="Ignored (1)"]').waitForClickable({ timeout: WAIT_TIMEOUT });
    await click('button[title="Ignored (1)"]');
    const ignored = await $('dialog[aria-labelledby="ignored-pairs-title"]');
    await ignored.waitForDisplayed({ timeout: WAIT_TIMEOUT });
    const row = await ignored.$('.//tr[.//td[normalize-space(.)="QACloneA"]]');
    await row.waitForDisplayed({ timeout: WAIT_TIMEOUT });
    await row.moveTo();
    await row.$(textButton('Recover')).waitForClickable({ timeout: WAIT_TIMEOUT });
    await row.$(textButton('Recover')).click();
    await expect(await ignored.$('p=No ignored pairs found')).toBeDisplayed();
    await ignored.$('.modal-action').$(textButton('Close')).click();
    await ignored.waitForDisplayed({ reverse: true, timeout: WAIT_TIMEOUT });
    const recoveredReport = await scanAndReadReport(fixture);
    const removedMember = recoveredReport.groups[0].members.find(
      (member) => member.displayName === 'QACloneB',
    );
    if (!removedMember) throw new Error('Recovered native duplicate report omitted QACloneB');
    await click('[role="button"][aria-label$=": QACloneA"]');
    resolution = await openResolution();
    await resolution.$(textButton('Confirm & Resolve')).click();
    await resolution.waitForDisplayed({ reverse: true, timeout: WAIT_TIMEOUT });
    await browser.waitUntil(
      async () =>
        !(await fs.readdir(path.join(fixture.game.modsPath, 'QACloneObject'))).includes('QACloneB'),
      {
        timeout: WAIT_TIMEOUT,
        timeoutMsg: 'Rendered Keep action did not remove generated QACloneB',
      },
    );
    console.log(
      `EMMM_QA_RECYCLE_CANDIDATE ${JSON.stringify({
        action: 'duplicate_keep_other_removed',
        originalPath: removedMember.folderPath,
        fixtureRoot: fixture.game.fixtureRoot,
      })}`,
    );
    expect(
      await fs.readFile(
        path.join(fixture.game.modsPath, 'QACloneObject', 'QACloneA', 'mod.ini'),
        'utf8',
      ),
    ).toBe('[TextureOverrideMockMod]\nhash = 0123456789abcdef\n');
    await expect(await $(NO_APPLY_ACTIONS)).toBeDisabled();
  });

  it('cancels an active native duplicate scan when Stop Scan is observable', async function () {
    const fixture = await startFixture('ScanCancel', true);
    await gotoWorkspaceView('storage-optimizer');
    await click(textButton('Start Scan'));
    await browser.waitUntil(
      async () =>
        (await $(textButton('Stop Scan')).isDisplayed()) ||
        (await $('table[aria-label="Duplicate Scan Results"]').isDisplayed()),
      { timeout: WAIT_TIMEOUT, timeoutMsg: 'Scan exposed neither progress nor results' },
    );
    const stop = await $(textButton('Stop Scan'));
    if (!(await stop.isDisplayed()) || !(await stop.isClickable())) {
      console.log(
        'EMMM_E2E_BLOCKED Stop Scan: tiny native fixture completed before the control was actionable',
      );
      this.skip();
    }
    await stop.click();
    await $(textButton('Start Scan')).waitForClickable({ timeout: WAIT_TIMEOUT });
    const completedReport = await invokeInApp<DupScanReport | null>('dup_scan_get_report', {
      gameId: fixture.gameId,
    });
    const visibleBody = await browser.execute(() => document.body.innerText.slice(-1_500));
    console.log(
      `EMMM_QA_SCAN_CANCEL_OBSERVATION ${JSON.stringify({ gameId: fixture.gameId, completedReport, visibleBody })}`,
    );
    if (completedReport) {
      console.log(
        'EMMM_E2E_INCONCLUSIVE Stop Scan: a completed report was persisted for the fresh tiny fixture before cancellation could be established',
      );
    }
    await expect(await $('*=Scan cancelled')).toBeDisplayed();
    expect((await fs.readdir(path.join(fixture.game.modsPath, 'QACloneObject'))).sort()).toEqual([
      'QACloneA',
      'QACloneB',
    ]);
  });

  it('uses Discover blank-tab actions, library toggles and address confirmation without remote navigation', async () => {
    await startFixture('Discover');
    const bookmarks = await invokeInApp<BrowserBookmark[]>('browser_list_bookmarks');
    expect(
      bookmarks.map((bookmark) => ({ id: bookmark.id, url: bookmark.url, title: bookmark.title })),
    ).toEqual([
      { id: 'default-gamebanana-bookmark', url: 'https://gamebanana.com/', title: 'GameBanana' },
    ]);
    await gotoWorkspaceView('browser');
    await $('section[aria-label="New Tab"]').waitForDisplayed({ timeout: WAIT_TIMEOUT });
    const initial = await browserTabCount();
    expect(initial).toBeGreaterThan(0);
    await expect(await $('button[title="Back"]')).toBeDisabled();
    await expect(await $('button[title="Forward"]')).toBeDisabled();
    await expect(await $('button[title="Refresh"]')).toBeDisabled();
    await expect(await $('button[aria-label="Add bookmark"]')).toBeDisabled();
    await expect(await $('section[aria-label="New Tab"]').$(textButton('Search'))).toBeDisabled();
    const search = await $('input#browser-new-tab-search');
    await search.setValue('   ');
    await expect(await $('section[aria-label="New Tab"]').$(textButton('Search'))).toBeDisabled();
    await click('button[aria-label="New Tab"]');
    await expectTabCount(initial + 1);
    await activeTabContext();
    await expect(await $(textButton('Reload tab'))).toBeDisabled();
    await click(textButton('Duplicate tab'));
    await expectTabCount(initial + 2);
    await activeTabContext();
    await click(textButton('Close tab'));
    await expectTabCount(initial + 1);
    await activeTabContext();
    await expect(await $(textButton('Reopen last closed tab'))).toBeDisabled();
    await click(textButton('Close tab'));
    await expectTabCount(initial);
    await openBrowserMenu();
    await $('[data-testid="browser-toolbar-menu"]').$(textButton('New Tab')).click();
    await expectTabCount(initial + 1);
    await click(
      './/div[button[@role="tab" and @aria-selected="true"]]/button[@aria-label="Close New Tab"]',
    );
    await expectTabCount(initial);

    await openBrowserMenu();
    let menu = await $('[data-testid="browser-toolbar-menu"]');
    for (const label of [
      'Find in page',
      'Clear cookies & site data',
      'Clear cache',
      'Open in default browser',
    ]) {
      await expect(
        await menu.$(`.//button[starts-with(normalize-space(.),"${label}")]`),
      ).toBeDisabled();
    }
    await expect(await menu.$('button[aria-label="Zoom out"]')).toBeDisabled();
    await expect(await menu.$('button[aria-label="Zoom in"]')).toBeDisabled();
    await menu.$(textButton('Bookmarks')).click();
    let library = await $('aside[aria-label="Bookmarks & history"]');
    await library.waitForDisplayed({ timeout: WAIT_TIMEOUT });
    await expect(
      await library.$('button[title="https://gamebanana.com/"] span:first-child'),
    ).toHaveText('GameBanana');
    await expect(await library.$('button[aria-label="Edit bookmark"]')).toBeDisplayed();
    await expect(await library.$('button[aria-label="Remove bookmark"]')).toBeDisplayed();
    await library.$('.//button[@role="tab" and normalize-space(.)="History"]').click();
    await expect(await library.$('p=No browsing history yet.')).toBeDisplayed();
    await click('button[aria-label="Close bookmarks and history"]');
    await library.waitForDisplayed({ reverse: true, timeout: WAIT_TIMEOUT });
    await click(textButton('Manage bookmarks'));
    library = await $('aside[aria-label="Bookmarks & history"]');
    await library.waitForDisplayed({ timeout: WAIT_TIMEOUT });
    await click('button[aria-label="Close bookmarks and history"]');
    await openBrowserMenu();
    menu = await $('[data-testid="browser-toolbar-menu"]');
    await menu.$(textButton('History')).click();
    library = await $('aside[aria-label="Bookmarks & history"]');
    await expect(await library.$('button[role="tab"][aria-selected="true"]')).toHaveText('History');
    await click('button[aria-label="Close bookmarks and history"]');

    const address = await $('input[placeholder="Enter URL to open..."]');
    await address.setValue('   ');
    await browser.keys('Enter');
    await expectTabCount(initial);
    await expect(await $('section[aria-label="New Tab"]')).toBeDisplayed();
    for (const value of [
      'https://',
      'http://127.0.0.1:9/QA-no-navigation',
      'https://qa-user@127.0.0.1/QA-no-navigation',
    ]) {
      const draft = await $('#browser-url-input');
      const startsAsInput = (await draft.getTagName()) === 'input';
      await draft.waitForClickable({ timeout: WAIT_TIMEOUT });
      await draft.click();
      if (startsAsInput) {
        await browser.keys(['Control', 'a']);
        await browser.keys(['Backspace']);
        await browser.keys(Array.from('http://1'));
        const formattedDraft = await $('button#browser-url-input');
        await formattedDraft.waitForClickable({ timeout: WAIT_TIMEOUT });
        await expect(formattedDraft).toHaveAttribute('title', 'http://1');
        console.log(
          'EMMM_QA_BROWSER_ENTRY_WORKAROUND blank input formatted before editing; trusted draft click used for security-confirmation coverage',
        );
        await formattedDraft.click();
      }
      const editableAddress = await $('input#browser-url-input');
      await editableAddress.waitForDisplayed({ timeout: WAIT_TIMEOUT });
      await browser.keys(['Control', 'a']);
      await browser.keys(['Backspace']);
      await browser.keys(Array.from(value));
      await expect(editableAddress).toHaveValue(value);
      await browser.keys(['Enter']);
      const confirmation = await $('dialog.modal[open]');
      await confirmation.waitForDisplayed({ timeout: WAIT_TIMEOUT });
      await expect(confirmation).toHaveText(/Confirm action/);
      await confirmation.$(textButton('Cancel')).click();
      await confirmation.waitForDisplayed({ reverse: true, timeout: WAIT_TIMEOUT });
      await expectTabCount(initial);
      await expect(await $('section[aria-label="New Tab"]')).toBeDisplayed();
    }
  });

  it('refreshes empty Downloads and navigates through Discover panel detail and Mod Inbox', async () => {
    await startFixture('Downloads');
    await gotoWorkspaceView('downloads');
    await expect(await $('h2=No downloads yet')).toBeDisplayed();
    await click('button[aria-label="Refresh downloads"]');
    await $('button[aria-label="Refresh downloads"]').waitForEnabled({ timeout: WAIT_TIMEOUT });
    await expect(await $('h2=No downloads yet')).toBeDisplayed();
    await click(textButton('Discover'));
    await expectView('browser');
    await click('button[aria-label="Open Downloads panel"]');
    let panel = await $('aside[aria-label="Downloads"]');
    await panel.waitForDisplayed({ timeout: WAIT_TIMEOUT });
    await expect(await panel.$('p=No downloads yet')).toBeDisplayed();
    await panel.$('button[aria-label="Refresh downloads"]').click();
    await panel
      .$('button[aria-label="Refresh downloads"]')
      .waitForEnabled({ timeout: WAIT_TIMEOUT });
    await click('button[aria-label="Close download panel"]');
    await panel.waitForDisplayed({ reverse: true, timeout: WAIT_TIMEOUT });
    await click('button[aria-label="Open Downloads panel"]');
    panel = await $('aside[aria-label="Downloads"]');
    await panel.waitForDisplayed({ timeout: WAIT_TIMEOUT });
    await panel.$(textButton('View detail')).click();
    await expectView('downloads');
    await expect(await $('h2=No downloads yet')).toBeDisplayed();
    await click(textButton('Open Mod Inbox'));
    await expectView('mod-inbox');
    await expect(await $('h2=Your Mod Inbox folder does not exist yet.')).toBeDisplayed();
  });

  it.skip(
    'BLOCKED: positive download, child-page reopen/find/zoom/history/bookmark and image/Base64 menus require a reachable owned page and WebView2 event setup',
  );
  it.skip(
    'BLOCKED: native picker, Open Inbox/Explorer, default browser and real-network links are outside this generated-fixture smoke scope',
  );
});
