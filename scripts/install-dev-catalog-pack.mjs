import { constants as fsConstants } from 'node:fs';
import { createHash } from 'node:crypto';
import { access, cp, mkdir, readFile, rename, rm } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = fileURLToPath(new URL('..', import.meta.url));
const devAppIdentifier = 'com.reynalivan.emmm.dev';
const packDirectory = 'asset-pack';
const packEntries = ['manifest.json', 'catalog', 'images'];

function hasErrorCode(error, code) {
  return typeof error === 'object' && error !== null && 'code' in error && error.code === code;
}

async function pathExists(path) {
  try {
    await access(path, fsConstants.F_OK);
    return true;
  } catch {
    return false;
  }
}

function resolveSourcePath() {
  return resolve(process.env.EMMM_DEV_CATALOG_SOURCE ?? join(projectRoot, '..', '3dm-catalog-asset'));
}

function resolveDestinationPath() {
  if (process.platform !== 'win32' || !process.env.APPDATA) return null;
  return join(process.env.APPDATA, devAppIdentifier, packDirectory);
}

function validateCatalogPath(path) {
  return (
    typeof path === 'string' &&
    path.startsWith('catalog/') &&
    !path.includes('..') &&
    !path.includes('\\')
  );
}

function hasText(value) {
  return typeof value === 'string' && value.trim().length > 0;
}

async function sha256File(path) {
  return createHash('sha256').update(await readFile(path)).digest('hex');
}

async function validateSource(source) {
  const manifestPath = join(source, 'manifest.json');
  let manifest;
  try {
    manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
  } catch (error) {
    throw new Error(`The development catalog manifest is invalid: ${String(error)}`);
  }

  if (
    manifest.format_version !== 1 ||
    !hasText(manifest.id) ||
    !hasText(manifest.version) ||
    !hasText(manifest.author) ||
    !hasText(manifest.source) ||
    !hasText(manifest.license) ||
    !manifest.catalogs ||
    typeof manifest.catalogs !== 'object'
  ) {
    throw new Error('The development catalog manifest has an unsupported format.');
  }

  for (const catalog of Object.values(manifest.catalogs)) {
    if (
      !catalog ||
      typeof catalog !== 'object' ||
      !validateCatalogPath(catalog.path) ||
      !/^[a-f0-9]{64}$/.test(catalog.sha256)
    ) {
      throw new Error('The development catalog manifest contains an unsafe catalog path.');
    }
    const catalogPath = join(source, catalog.path);
    if (!(await pathExists(catalogPath))) {
      throw new Error(`The development catalog is missing ${catalog.path}.`);
    }
    if ((await sha256File(catalogPath)) !== catalog.sha256) {
      throw new Error(`The development catalog checksum does not match for ${catalog.path}.`);
    }
  }
}

async function copyPack(source, destination) {
  const stage = `${destination}.staging-${process.pid}`;
  const backup = `${destination}.backup-${process.pid}`;
  let movedCurrentPack = false;

  await rm(stage, { recursive: true, force: true });
  await rm(backup, { recursive: true, force: true });
  await mkdir(stage, { recursive: true });

  try {
    for (const entry of packEntries) {
      const sourceEntry = join(source, entry);
      if (await pathExists(sourceEntry)) {
        await cp(sourceEntry, join(stage, entry), { recursive: true });
      }
    }

    await mkdir(resolve(destination, '..'), { recursive: true });
    if (await pathExists(destination)) {
      await rename(destination, backup);
      movedCurrentPack = true;
    }
    await rename(stage, destination);
    if (movedCurrentPack) await rm(backup, { recursive: true, force: true });
  } catch (error) {
    if (movedCurrentPack && !(await pathExists(destination)) && (await pathExists(backup))) {
      await rename(backup, destination).catch(() => undefined);
    }
    throw error;
  } finally {
    await rm(stage, { recursive: true, force: true });
  }
}

export async function installDevCatalogPack() {
  const destination = resolveDestinationPath();
  if (!destination) {
    console.info('Skipping development catalog pack installation outside Windows.');
    return;
  }

  const source = resolveSourcePath();
  if (!(await pathExists(source))) {
    throw new Error(
      `Development catalog source not found at ${source}. Clone ../3dm-catalog-asset or set EMMM_DEV_CATALOG_SOURCE.`,
    );
  }

  await validateSource(source);
  await copyPack(source, destination);
  console.info(`Installed development catalog pack at ${destination}.`);
}
