import { browser, $, expect } from '@wdio/globals';
import fs from 'node:fs/promises';
import path from 'node:path';
import { createServer, type Server, type ServerResponse } from 'node:http';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { createMockGame, scheduleMockGameRemoval, type MockGame } from '../support/fixtures.js';
import { waitForWorkspaceCoreReady } from '../support/data.js';
import { invokeInApp, invokeWithChannel } from '../support/ipc.js';
import type {
  AppSettings,
  BrowserDownloadDto,
  DupScanReport,
  ResolutionSummary,
} from '../../../src/shared/api/tauri/bindings.gen.js';

interface DownloadConfirmationRequest {
  id: string;
  filename: string;
  source_url: string;
  destination_path: string;
}

interface HistoryEntry {
  url: string;
  title: string;
  visit_count: number;
}

interface CaptureWindow extends Window {
  __EMMM_DOWNLOAD_REQUESTS__: DownloadConfirmationRequest[];
  __TAURI__: {
    event: {
      listen: <T>(event: string, handler: (event: { payload: T }) => void) => Promise<() => void>;
    };
    webview: {
      Webview: { getByLabel: (label: string) => Promise<{ close: () => Promise<void> } | null> };
    };
  };
}

const ZIP = Buffer.from(`504b0506${'00'.repeat(18)}`, 'hex');
const MOD_INI = '[TextureOverrideNativeFixture]\nhash = 21222324\n';

describe('Native browser and duplicates regressions', () => {
  let game: MockGame;
  let gameId: string;
  let inbox: string;
  let server: Server;
  let origin: string;
  let assetResponse: ServerResponse | undefined;
  let slowResponse: ServerResponse | undefined;
  let holdFirstAsset = true;
  const labels = new Set<string>();

  async function closeTab(label: string): Promise<void> {
    const result = await browser.executeAsync(
      async (value: string, done: (result: string) => void) => {
        try {
          const view = await (
            window as unknown as CaptureWindow
          ).__TAURI__.webview.Webview.getByLabel(value);
          if (view) await view.close();
          done('closed');
        } catch (error: unknown) {
          done(error instanceof Error ? error.message : String(error));
        }
      },
      label,
    );
    expect(result).toBe('closed');
    labels.delete(label);
  }

  async function openTab(url: string): Promise<string> {
    const label = await invokeInApp<string>('browser_open_tab', { url, sessionId: null });
    labels.add(label);
    return label;
  }

  async function resetCapture(): Promise<void> {
    await browser.execute(() => {
      (window as unknown as CaptureWindow).__EMMM_DOWNLOAD_REQUESTS__ = [];
    });
  }

  async function requestFor(url: string): Promise<DownloadConfirmationRequest> {
    let captured: DownloadConfirmationRequest | null = null;
    await browser.waitUntil(
      async () => {
        captured = await browser.execute((source: string) => {
          const requests = (window as unknown as CaptureWindow).__EMMM_DOWNLOAD_REQUESTS__;
          return requests.find((request) => request.source_url === source) ?? null;
        }, url);
        return captured !== null;
      },
      { timeout: 15000, interval: 100, timeoutMsg: `No native confirmation for ${url}` },
    );
    if (!captured) throw new Error('Native confirmation disappeared');
    const request: DownloadConfirmationRequest = captured;
    const relative = path.relative(inbox, request.destination_path);
    expect(relative.startsWith('..') || path.isAbsolute(relative)).toBe(false);
    expect(relative.length).toBeGreaterThan(0);
    await $('dialog[aria-labelledby="download-confirmation-title"]').waitForDisplayed();
    return request;
  }

  async function acceptConfirmation(): Promise<void> {
    await $('dialog[aria-labelledby="download-confirmation-title"] .btn-primary').click();
  }

  async function downloadWithStatus(
    id: string,
    statuses: readonly string[],
  ): Promise<BrowserDownloadDto> {
    let found: BrowserDownloadDto | undefined;
    await browser.waitUntil(
      async () => {
        const downloads = await invokeInApp<BrowserDownloadDto[]>('browser_list_downloads', {
          gameId,
        });
        found = downloads.find((item) => item.id === id);
        return found !== undefined && statuses.includes(found.status);
      },
      {
        timeout: 20000,
        interval: 100,
        timeoutMsg: `Download ${id} did not settle to ${statuses.join('/')}`,
      },
    );
    if (!found) throw new Error('Native download row disappeared');
    return found;
  }

  async function scan(): Promise<DupScanReport> {
    const previous = await invokeInApp<DupScanReport | null>('dup_scan_get_report', { gameId });
    await invokeWithChannel('dup_scan_start', { gameId, modsRoot: game.modsPath }, 'onEvent');
    let report: DupScanReport | null = null;
    await browser.waitUntil(
      async () => {
        report = await invokeInApp<DupScanReport | null>('dup_scan_get_report', { gameId });
        return report !== null && report.scanId !== previous?.scanId;
      },
      { timeout: 20000, interval: 100, timeoutMsg: 'Duplicate report did not finish' },
    );
    if (!report) throw new Error('Duplicate report missing after completion');
    return report;
  }

  before(async () => {
    expect(await invokeInApp<string>('plugin:app|identifier')).toBe('com.reynalivan.emmm.e2e');
    game = await createMockGame('BrowserDuplicatesRegressions');
    inbox = path.join(game.root, 'Inbox');
    await fs.mkdir(inbox);
    for (const name of ['CloneA', 'CloneB']) {
      const folder = path.join(game.modsPath, 'Clones', name);
      await fs.mkdir(folder, { recursive: true });
      await fs.writeFile(path.join(folder, 'mod.ini'), MOD_INI);
      await fs.writeFile(path.join(folder, 'body.dds'), 'same clone bytes');
    }
    for (const [name, asset] of [
      ['FalseExactA', 'a.dds'],
      ['FalseExactB', 'b.dds'],
    ] as const) {
      const folder = path.join(game.modsPath, 'FalseExact', name);
      await fs.mkdir(folder, { recursive: true });
      await fs.writeFile(
        path.join(folder, 'mod.ini'),
        '[TextureOverrideNativeFixture]\nhash = 10111213\n',
      );
      await fs.writeFile(path.join(folder, asset), name.endsWith('A') ? 'AAA' : 'BBB');
    }
    gameId = await seedGameAndOpenDashboard(game);
    const settings = await invokeInApp<AppSettings>('get_settings');
    await invokeInApp('save_settings', {
      settings: {
        ...settings,
        games: settings.games.map((entry) =>
          entry.id === gameId ? { ...entry, ready_to_move_path: inbox } : entry,
        ),
      },
    });
    await invokeInApp('browser_set_adblock_enabled', { enabled: false });
    const registered = await browser.executeAsync((done: (value: boolean) => void) => {
      const target = window as unknown as CaptureWindow;
      target.__EMMM_DOWNLOAD_REQUESTS__ = [];
      target.__TAURI__.event
        .listen<DownloadConfirmationRequest>('browser:download-confirmation-requested', (event) => {
          target.__EMMM_DOWNLOAD_REQUESTS__.push(event.payload);
        })
        .then(
          () => done(true),
          () => done(false),
        );
    });
    expect(registered).toBe(true);
    server = createServer((request, response) => {
      if (request.url === '/title') {
        response.writeHead(200, { 'Content-Type': 'text/html' });
        response.end('<title>Native stable title</title><img src="/asset">');
      } else if (request.url === '/asset') {
        if (holdFirstAsset) {
          assetResponse = response;
        } else {
          response.writeHead(200, { 'Content-Type': 'image/svg+xml' });
          response.end('<svg xmlns="http://www.w3.org/2000/svg"/>');
        }
      } else if (request.url === '/slow.zip') {
        response.writeHead(200, {
          'Content-Type': 'application/zip',
          'Content-Length': 1048576,
          'Content-Disposition': 'attachment; filename="slow.zip"',
        });
        response.write(Buffer.alloc(65536));
        slowResponse = response;
      } else {
        const disposition =
          request.url === '/encoded.zip'
            ? "attachment; filename*=UTF-8''Ayaka%20mod.zip"
            : 'attachment; filename="collision.zip"';
        response.writeHead(200, {
          'Content-Type': 'application/zip',
          'Content-Length': ZIP.length,
          'Content-Disposition': disposition,
        });
        response.end(request.method === 'HEAD' ? undefined : ZIP);
      }
    });
    await new Promise<void>((resolve, reject) => {
      server.once('error', reject);
      server.listen(0, '127.0.0.1', resolve);
    });
    const address = server.address();
    if (!address || typeof address === 'string')
      throw new Error('Fixture server has no TCP address');
    origin = `http://127.0.0.1:${address.port}`;
  });

  after(async () => {
    assetResponse?.destroy();
    slowResponse?.destroy();
    for (const label of [...labels]) await closeTab(label);
    if (server) {
      server.closeAllConnections();
      await new Promise<void>((resolve, reject) =>
        server.close((error) => (error ? reject(error) : resolve())),
      );
    }
    if (game) scheduleMockGameRemoval(game);
  });

  it('keeps an early native title through Finished and a revisit', async () => {
    const url = `${origin}/title`;
    const label = await openTab(url);
    await browser.waitUntil(
      async () => {
        const history = await invokeInApp<HistoryEntry[]>('browser_list_history', { limit: 100 });
        return history.some((entry) => entry.url === url && entry.title === 'Native stable title');
      },
      { timeout: 10000, interval: 100 },
    );
    expect(assetResponse).toBeDefined();
    holdFirstAsset = false;
    assetResponse!.writeHead(200, { 'Content-Type': 'image/svg+xml' });
    assetResponse!.end('<svg xmlns="http://www.w3.org/2000/svg"/>');
    await browser.waitUntil(
      async () =>
        (await invokeInApp<HistoryEntry[]>('browser_list_history', { limit: 100 })).some(
          (entry) =>
            entry.url === url && entry.title === 'Native stable title' && entry.visit_count === 1,
        ),
      { timeout: 10000, interval: 100 },
    );
    await invokeInApp('browser_reload_tab', { label });
    await browser.waitUntil(
      async () =>
        (await invokeInApp<HistoryEntry[]>('browser_list_history', { limit: 100 })).some(
          (entry) =>
            entry.url === url && entry.title === 'Native stable title' && entry.visit_count === 2,
        ),
      { timeout: 10000, interval: 100 },
    );
    await closeTab(label);
  });

  it('decodes the native extended filename and publishes only the completed archive', async () => {
    await resetCapture();
    const url = `${origin}/encoded.zip`;
    const label = await openTab(url);
    const request = await requestFor(url);
    expect(request.filename).toBe('Ayaka mod.zip');
    await acceptConfirmation();
    const row = await downloadWithStatus(request.id, ['finished']);
    expect(row.file_path).toBe(request.destination_path);
    expect(row.bytes_received).toBe(ZIP.length);
    expect(await fs.readFile(request.destination_path)).toEqual(ZIP);
    await closeTab(label);
  });

  it('preserves a raced destination and reports complete counters on the fast fallback retry', async () => {
    await resetCapture();
    const url = `${origin}/collision.zip`;
    const label = await openTab(url);
    const request = await requestFor(url);
    await fs.writeFile(request.destination_path, 'intervening user file', { flag: 'wx' });
    await acceptConfirmation();
    const failed = await downloadWithStatus(request.id, ['failed']);
    expect(failed.can_resume).toBe(false);
    expect(failed.file_path).not.toBe(request.destination_path);
    expect(await fs.readFile(request.destination_path, 'utf8')).toBe('intervening user file');
    await resetCapture();
    await invokeInApp('browser_retry_download', { id: request.id });
    const retry = await requestFor(url);
    expect(retry.id).not.toBe(request.id);
    expect(retry.destination_path).not.toBe(request.destination_path);
    await acceptConfirmation();
    let completed: BrowserDownloadDto | undefined;
    await browser.waitUntil(
      async () => {
        completed = (
          await invokeInApp<BrowserDownloadDto[]>('browser_list_downloads', { gameId })
        ).find((row) => row.file_path === retry.destination_path && row.source_url === url);
        if (completed?.status === 'failed') {
          throw new Error(
            `Fallback retry failed: ${completed.error_msg ?? 'missing native error'}`,
          );
        }
        return completed?.status === 'finished';
      },
      {
        timeout: 20000,
        interval: 100,
        timeoutMsg: 'Fallback retry worker did not finish its confirmed destination',
      },
    );
    if (!completed) throw new Error('Fallback retry completed without its worker row');
    expect(completed.id).not.toBe(request.id);
    expect(completed.bytes_received).toBe(ZIP.length);
    expect(completed.bytes_total).toBe(ZIP.length);
    expect(await fs.readFile(retry.destination_path)).toEqual(ZIP);
    expect(await fs.readFile(request.destination_path, 'utf8')).toBe('intervening user file');
    await closeTab(label);
  });

  it('settles a closed native source tab without resurrecting resumable history', async () => {
    await resetCapture();
    const url = `${origin}/slow.zip`;
    const label = await openTab(url);
    const request = await requestFor(url);
    await acceptConfirmation();
    await downloadWithStatus(request.id, ['in_progress']);
    await closeTab(label);
    slowResponse?.destroy();
    await downloadWithStatus(request.id, ['failed', 'canceled']);
    let stableSince: number | null = null;
    await browser.waitUntil(
      async () => {
        const row = (
          await invokeInApp<BrowserDownloadDto[]>('browser_list_downloads', { gameId })
        ).find((item) => item.id === request.id);
        expect(row).toBeDefined();
        expect(row!.can_resume).toBe(false);
        expect(['failed', 'canceled']).toContain(row!.status);
        stableSince ??= Date.now();
        return Date.now() - stableSince >= 1500;
      },
      { timeout: 10000, interval: 100 },
    );
  });

  it('preserves indexed physical identities and excludes false exact assets', async () => {
    const report = await scan();
    const clones = report.groups.find(
      (group) =>
        group.confidenceScore === 100 &&
        group.members.some((member) => member.folderPath.endsWith(path.join('Clones', 'CloneA'))),
    );
    expect(clones).toBeDefined();
    expect(clones!.members).toHaveLength(2);
    expect(new Set(clones!.members.map((member) => member.modId)).size).toBe(2);
    const enabled = clones!.members.find((member) =>
      member.folderPath.endsWith(path.join('Clones', 'CloneA')),
    )!;
    const other = clones!.members.find((member) =>
      member.folderPath.endsWith(path.join('Clones', 'CloneB')),
    )!;
    expect(enabled.modId).not.toBeNull();
    expect(other.modId).not.toBeNull();
    expect(
      report.groups.filter(
        (group) =>
          group.confidenceScore === 100 &&
          group.members.some((member) => member.folderPath.includes('FalseExact')),
      ),
    ).toHaveLength(0);
    const resolution = await invokeInApp<ResolutionSummary>('dup_resolve_batch', {
      gameId,
      requests: [
        {
          groupId: clones!.groupId,
          action: 'ignore',
          folderA: enabled.folderPath,
          folderB: other.folderPath,
        },
      ],
    });
    expect(resolution.successful).toBe(1);
  });

  it('reports all five allowed exact pairs after ignoring only one of four clones', async () => {
    for (const name of ['CloneC', 'CloneD']) {
      const folder = path.join(game.modsPath, 'Clones', name);
      await fs.mkdir(folder);
      await fs.writeFile(path.join(folder, 'mod.ini'), MOD_INI);
      await fs.writeFile(path.join(folder, 'body.dds'), 'same clone bytes');
    }
    await invokeInApp('reconcile_disk_state_cmd', {
      gameId,
      reason: 'ManualRepair',
      changedPaths: [],
      forceFull: true,
    });
    await waitForWorkspaceCoreReady(gameId);
    const report = await scan();
    const pairs = report.groups.filter(
      (group) =>
        group.confidenceScore === 100 &&
        group.members.every((member) => member.folderPath.includes(`${path.sep}Clones${path.sep}`)),
    );
    expect(pairs).toHaveLength(5);
    expect(pairs.every((group) => group.members.length === 2)).toBe(true);
  });
});
