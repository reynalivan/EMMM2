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
 */
export async function createMockGame(label = 'E2E'): Promise<MockGame> {
  const root = path.join(os.tmpdir(), `EMMM_${label}_${Date.now()}`);
  const modsPath = path.join(root, 'Mods');
  const exePath = path.join(root, 'Fake_Game_Loader.exe');

  await fs.mkdir(modsPath, { recursive: true });
  await fs.writeFile(exePath, 'mock exe binary content');
  await fs.writeFile(path.join(root, 'd3dx.ini'), '[Main]\n');
  await fs.writeFile(path.join(root, 'd3d11.dll'), 'mock dll content');

  return { root, modsPath, exePath };
}

/**
 * Adds a mod folder at `Mods/<object>/<mod>/` with a stub `mod.ini` so it is a
 * recognizable mod on disk. Returns the absolute mod folder path.
 */
export async function addMockMod(game: MockGame, object: string, mod: string): Promise<string> {
  const modPath = path.join(game.modsPath, object, mod);
  await fs.mkdir(modPath, { recursive: true });
  await fs.writeFile(path.join(modPath, 'mod.ini'), '[Constants]\n');
  return modPath;
}

export async function removeMockGame(game: MockGame): Promise<void> {
  await fs.rm(game.root, { recursive: true, force: true });
}

/** Directory listing that returns `[]` instead of throwing when the dir is gone. */
export async function listDir(dir: string): Promise<string[]> {
  try {
    return await fs.readdir(dir);
  } catch {
    return [];
  }
}
