import { expect } from '@wdio/globals';
import fs from 'fs/promises';
import path from 'path';
import { createMockGame, removeMockGame, listDir, type MockGame } from '../support/fixtures.js';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { invokeInApp, invokeWithChannel } from '../support/ipc.js';

interface BulkResult {
  success: string[];
  failures: { path: string; error: unknown }[];
}
interface IngestResult {
  moved: string[];
  failed: string[];
  skipped: string[];
}
interface ArchiveInfo {
  path: string;
  name: string;
  [key: string]: unknown;
}

const FIXTURE_ZIP = path.resolve('test/fixtures/sample-mod.zip');

/** Creates a loose folder (outside the mods tree) with a stub mod.ini. */
async function makeLooseFolder(root: string, name: string): Promise<string> {
  const dir = path.join(root, 'incoming', name);
  await fs.mkdir(dir, { recursive: true });
  await fs.writeFile(path.join(dir, 'mod.ini'), '[Constants]\n');
  return dir;
}

/**
 * Fase 5 — Import & organisasi ⚠️ DATA-SAFETY. Folder import, auto-organizer
 * ingest, collision handling, and real archive extraction (plain zip fixture).
 * Password/7z archives remain [manual-smoke].
 */
describe('Fase 5 — Import & Organization (data-safety)', () => {
  let game: MockGame;
  let gameId: string;

  before(async () => {
    game = await createMockGame('Phase5');
    gameId = await seedGameAndOpenDashboard(game);
  });

  after(async () => {
    await removeMockGame(game);
  });

  it('TC-23-01: Import moves a loose folder into the mods library', async () => {
    const loose = await makeLooseFolder(game.root, 'ImportedMod');
    const res = await invokeInApp<BulkResult>('import_mods_from_paths', {
      paths: [loose],
      targetDir: game.modsPath,
      strategy: 'Raw',
      dbJson: null,
    });
    expect(res.failures.length).toBe(0);
    expect(await listDir(game.modsPath)).toContain('ImportedMod');
  });

  it('TC-39-01: Import into an existing name reports collision, source untouched', async () => {
    await fs.mkdir(path.join(game.modsPath, 'DupName'), { recursive: true });
    const loose = await makeLooseFolder(game.root, 'DupName');

    const res = await invokeInApp<BulkResult>('import_mods_from_paths', {
      paths: [loose],
      targetDir: game.modsPath,
      strategy: 'Raw',
      dbJson: null,
    });
    expect(res.failures.length).toBe(1);
    // Source folder must survive a rejected import (no data loss).
    expect(await listDir(loose)).toContain('mod.ini');
  });

  it('TC-38-01: Auto-organizer ingest relocates dropped folders', async () => {
    const loose = await makeLooseFolder(game.root, 'DroppedMod');
    const res = await invokeInApp<IngestResult>('ingest_dropped_folders', {
      paths: [loose],
      modsPath: game.modsPath,
      gameId,
      gameName: 'E2E',
      gameType: 'GIMI',
    });
    expect(res.moved.length + res.skipped.length).toBeGreaterThan(0);
    expect(res.failed.length).toBe(0);
  });

  it('TC-37-01: Archive extraction unpacks a zip into the library', async () => {
    const scanDir = path.join(game.root, 'archives');
    await fs.mkdir(scanDir, { recursive: true });
    const zipCopy = path.join(scanDir, 'sample-mod.zip');
    await fs.copyFile(FIXTURE_ZIP, zipCopy);

    const detected = await invokeInApp<ArchiveInfo[]>('detect_archives_cmd', {
      modsPath: scanDir,
    });
    expect(detected.some((a) => a.name.includes('sample-mod'))).toBe(true);

    await invokeWithChannel<unknown>(
      'extract_archive_cmd',
      {
        archivePath: zipCopy,
        modsDir: game.modsPath,
        overwrite: false,
        disableAfter: false,
        unpackNested: false,
      },
      'onProgress',
    );

    expect(await listDir(game.modsPath)).toContain('ArchivedMod');
  });
});
