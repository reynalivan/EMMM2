import fs from 'fs/promises';
import path from 'path';
import os from 'os';

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
  const root = atRoot ?? path.join(os.tmpdir(), `EMMM_${label}_${Date.now()}`);
  const modsPath = path.join(root, 'Mods');
  const exePath = path.join(root, 'Fake_Game_Loader.exe');

  await fs.mkdir(modsPath, { recursive: true });
  await fs.writeFile(exePath, 'mock exe binary content');
  await fs.writeFile(path.join(root, 'd3dx.ini'), '[Main]\n');
  await fs.writeFile(path.join(root, 'd3d11.dll'), 'mock dll content');

  return { root, modsPath, exePath };
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

export async function removeMockGame(game: MockGame): Promise<void> {
  // The app keeps writing into `Mods/.emmm_data` (watcher, keybinds) as the
  // spec tears down, and on Windows that races `rm -r` into ENOTEMPTY/EBUSY.
  // Node retries the whole walk on those two errors specifically.
  await fs.rm(game.root, { recursive: true, force: true, maxRetries: 10, retryDelay: 100 });
}

/** Directory listing that returns `[]` instead of throwing when the dir is gone. */
export async function listDir(dir: string): Promise<string[]> {
  try {
    return await fs.readdir(dir);
  } catch {
    return [];
  }
}
