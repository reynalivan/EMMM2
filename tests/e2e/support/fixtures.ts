import fs from 'fs/promises';
import path from 'path';
import os from 'os';
import { randomUUID } from 'crypto';

const FIXTURE_MARKER = '.emmm-e2e-fixture';
const FIXTURE_ROOT_PREFIX = 'EMMM_';
const ownedFixtureRoots = new Map<string, { cleanupToken: string; references: number }>();
const scheduledRemovals = new Set<MockGame>();

/**
 * Shared E2E fixtures. Every spec that touches disk must build an isolated
 * mock game here (never the real library) and tear it down in `after()`.
 */

export interface MockGame {
  /** Game root folder (contains the 3DMigoto core files). */
  root: string;
  /** `<root>/Mods` — where object/mod folders live. */
  modsPath: string;
  /** Fake loader exe (name contains "loader" so the backend validator passes). */
  exePath: string;
  /** Owned root whose marker must match before cleanup is allowed. */
  fixtureRoot: string;
  /** Per-fixture cleanup proof, never persisted outside the owned root. */
  cleanupToken: string;
}

function isPathInside(parent: string, candidate: string): boolean {
  const relative = path.relative(parent, candidate);
  return relative !== '' && !relative.startsWith('..') && !path.isAbsolute(relative);
}

function ownedRootForExactPath(exactRoot: string): string {
  const tempRoot = path.resolve(os.tmpdir());
  const resolved = path.resolve(exactRoot);
  if (!isPathInside(tempRoot, resolved)) {
    throw new Error(`E2E fixture path must be inside the OS temp directory: ${resolved}`);
  }

  const [ownedDirectory] = path.relative(tempRoot, resolved).split(path.sep);
  if (!ownedDirectory?.startsWith(FIXTURE_ROOT_PREFIX)) {
    throw new Error(`E2E fixture path must be nested under an ${FIXTURE_ROOT_PREFIX}* root`);
  }
  return path.join(tempRoot, ownedDirectory);
}

/**
 * Creates an isolated mock game folder in the OS temp dir with the core files
 * the backend validator requires (loader exe, d3dx.ini, d3d11.dll). Caller
 * MUST call {@link removeMockGame} in `after()`.
 *
 * Pass `atRoot` to place the instance at an exact path — auto-detect scans for
 * `<root>/<GAMETYPE>/` (GIMI, SRMI, …), so that layout has to be built by hand.
 */
export async function createMockGame(label = 'E2E', atRoot?: string): Promise<MockGame> {
  const safeLabel = label.replace(/[^a-zA-Z0-9_-]/g, '_');
  const fixtureRoot = atRoot
    ? ownedRootForExactPath(atRoot)
    : await fs.mkdtemp(path.join(os.tmpdir(), `${FIXTURE_ROOT_PREFIX}${safeLabel}_`));
  let ownership = ownedFixtureRoots.get(fixtureRoot);
  if (!ownership) {
    if (atRoot) {
      await fs.mkdir(fixtureRoot);
    }
    ownership = { cleanupToken: randomUUID(), references: 0 };
    await fs.writeFile(path.join(fixtureRoot, FIXTURE_MARKER), ownership.cleanupToken, {
      encoding: 'utf8',
      flag: 'wx',
    });
    ownedFixtureRoots.set(fixtureRoot, ownership);
  }
  ownership.references += 1;

  const root = atRoot ? path.resolve(atRoot) : fixtureRoot;
  const modsPath = path.join(root, 'Mods');
  const exePath = path.join(root, 'Fake_Game_Loader.exe');

  await fs.mkdir(modsPath, { recursive: true });
  await fs.writeFile(exePath, 'mock exe binary content');
  await fs.writeFile(path.join(root, 'd3dx.ini'), '[Main]\n');
  await fs.writeFile(path.join(root, 'd3d11.dll'), 'mock dll content');

  return { root, modsPath, exePath, fixtureRoot, cleanupToken: ownership.cleanupToken };
}

/**
 * The classifier only counts a folder as a mod when one of its `.ini` files has
 * a `textureoverride` / `shaderoverride` / `resource` section. A `[Constants]`
 * stub reads as a plain container, so disk reconcile indexes nothing and every
 * `mod_count` / `enabled_count` / collection assertion sees zero.
 */
const MOD_INI = '[TextureOverrideMockMod]\nhash = 0123456789abcdef\n';

/**
 * Adds a mod folder at `Mods/<object>/<mod>/` with a `mod.ini` the classifier
 * recognizes as a mod. Returns the absolute mod folder path.
 */
export async function addMockMod(game: MockGame, object: string, mod: string): Promise<string> {
  const modPath = path.join(game.modsPath, object, mod);
  await fs.mkdir(modPath, { recursive: true });
  await fs.writeFile(path.join(modPath, 'mod.ini'), MOD_INI);
  return modPath;
}

/** Adds a large classifier-valid corpus without an unbounded Promise fan-out. */
export async function addMockMods(
  game: MockGame,
  object: string,
  mods: readonly string[],
  batchSize = 128,
): Promise<string[]> {
  if (!Number.isInteger(batchSize) || batchSize <= 0) {
    throw new Error('Mock mod batchSize must be a positive integer');
  }
  const paths: string[] = [];
  for (let offset = 0; offset < mods.length; offset += batchSize) {
    const batch = mods.slice(offset, offset + batchSize);
    paths.push(...(await Promise.all(batch.map((mod) => addMockMod(game, object, mod)))));
  }
  return paths;
}

export async function removeMockGame(game: MockGame): Promise<void> {
  const ownership = ownedFixtureRoots.get(game.fixtureRoot);
  if (!ownership || ownership.cleanupToken !== game.cleanupToken) {
    throw new Error(`Refusing to clean an unregistered fixture root: ${game.fixtureRoot}`);
  }
  ownership.references -= 1;
  if (ownership.references > 0) {
    return;
  }

  const tempRoot = await fs.realpath(os.tmpdir());
  const fixtureRoot = await fs.realpath(game.fixtureRoot);
  if (!isPathInside(tempRoot, fixtureRoot)) {
    throw new Error(`Refusing to clean fixture outside the OS temp directory: ${fixtureRoot}`);
  }
  if (!path.basename(fixtureRoot).startsWith(FIXTURE_ROOT_PREFIX)) {
    throw new Error(`Refusing to clean unowned fixture root: ${fixtureRoot}`);
  }

  const marker = await fs.readFile(path.join(fixtureRoot, FIXTURE_MARKER), 'utf8');
  if (marker !== game.cleanupToken) {
    throw new Error(`Refusing to clean fixture with a mismatched ownership marker: ${fixtureRoot}`);
  }

  // The app keeps writing into `Mods/.emmm_data` (watcher, keybinds) as the
  // spec tears down, and on Windows that races `rm -r` into ENOTEMPTY/EBUSY.
  // Node retries the whole walk on those two errors specifically.
  await fs.rm(fixtureRoot, { recursive: true, force: true, maxRetries: 10, retryDelay: 100 });
  ownedFixtureRoots.delete(game.fixtureRoot);
}

/** Native specs defer deletion until the harness has stopped its process tree. */
export function scheduleMockGameRemoval(game: MockGame): void {
  scheduledRemovals.add(game);
}

/** Called only after successful owned native process-tree termination. */
export async function cleanupScheduledMockGames(): Promise<number> {
  let removed = 0;
  for (const game of scheduledRemovals) {
    await removeMockGame(game);
    scheduledRemovals.delete(game);
    removed += 1;
  }
  return removed;
}

/** Directory listing that returns `[]` instead of throwing when the dir is gone. */
export async function listDir(dir: string): Promise<string[]> {
  try {
    return await fs.readdir(dir);
  } catch {
    return [];
  }
}
