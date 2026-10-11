import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { fileURLToPath } from 'node:url';

const OBJECT_COUNT = 148;
const MOD_COUNT = 4_243;
const OWNER_FILE = '.ignored-audit-owner.json';
const workspace = fs.realpathSync(path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..'));

function rejectLinkedAncestors(target) {
  for (let current = target; ; current = path.dirname(current)) {
    let metadata;
    try {
      metadata = fs.lstatSync(current);
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
    }
    assert(!metadata?.isSymbolicLink(), `Linked audit ancestor refused: ${current}`);
    if (path.dirname(current) === current) break;
  }
}

function writeExclusive(filename, content) {
  fs.writeFileSync(filename, content, { flag: 'wx' });
}

function folderKey(relative) {
  return relative
    .split('/')
    .map((component) =>
      component
        .replace(/^disabled[\s_-]*/i, '')
        .trim()
        .toLowerCase(),
    )
    .join('/');
}

function createDatabase(filename, gameRoot, modsRoot, objects, mods) {
  writeExclusive(filename, '');
  const database = new DatabaseSync(filename);
  try {
    const migrationRoot = path.join(workspace, 'src-tauri', 'migrations');
    const migrations = fs
      .readdirSync(migrationRoot)
      .filter((name) => /^\d+_.+\.sql$/.test(name))
      .sort();
    assert(migrations.length > 0, 'Canonical migrations missing');
    for (const migration of migrations) {
      database.exec(fs.readFileSync(path.join(migrationRoot, migration), 'utf8'));
    }
    database.exec('BEGIN IMMEDIATE');
    const gameId = 'ignored-audit-synthetic-game';
    database
      .prepare('INSERT INTO games (id, game_type, name, path, mods_path) VALUES (?, ?, ?, ?, ?)')
      .run(gameId, 0, 'Owned ignored-test fixture', gameRoot, modsRoot);
    const insertObject = database.prepare(`INSERT INTO objects
      (id, game_id, name, name_key, folder_path, folder_path_key, status, object_type)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?)`);
    for (const object of objects) {
      insertObject.run(
        object.id,
        gameId,
        object.name,
        object.name.toLowerCase(),
        object.folder,
        folderKey(object.folder),
        object.disabled ? 0 : 1,
        'Character',
      );
    }
    const insertMod = database.prepare(`INSERT INTO mods
      (id, game_id, object_id, folder_path, folder_path_key, actual_name, status,
       object_type, is_safe, safety_source, is_favorite, is_pinned)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`);
    for (const mod of mods) {
      insertMod.run(
        mod.id,
        gameId,
        mod.objectId,
        mod.folder,
        folderKey(mod.folder),
        mod.name,
        mod.disabled ? 0 : 1,
        'Character',
        mod.info.is_safe ? 1 : 0,
        'metadata',
        mod.info.is_favorite ? 1 : 0,
        mod.info.is_pinned ? 1 : 0,
      );
    }
    database.exec('COMMIT');
    assert.equal(database.prepare('SELECT COUNT(*) AS n FROM objects').get().n, OBJECT_COUNT);
    assert.equal(database.prepare('SELECT COUNT(*) AS n FROM mods').get().n, MOD_COUNT);
    assert.equal(database.prepare('PRAGMA integrity_check').get().integrity_check, 'ok');
    assert.deepEqual(database.prepare('PRAGMA foreign_key_check').all(), []);
  } finally {
    database.close();
  }
}

export function prepareIgnoredAuditFixture() {
  const runId = randomUUID();
  const cleanupToken = randomUUID();
  const auditBase = path.join(workspace, '.audit-output', 'ignored-tests');
  rejectLinkedAncestors(auditBase);
  fs.mkdirSync(auditBase, { recursive: true });
  const auditDirectory = path.join(auditBase, runId);
  fs.mkdirSync(auditDirectory);
  const tempBase = fs.realpathSync(os.tmpdir());
  const fixtureHome = fs.mkdtempSync(path.join(tempBase, `EMMM-ignored-tests-${runId}-`));
  assert.equal(path.dirname(fs.realpathSync(fixtureHome)), tempBase);
  const ownership = {
    kind: 'EMMM ignored-test synthetic fixture',
    runId,
    cleanupToken,
    workspace,
    fixtureHome,
    auditDirectory,
  };
  for (const directory of [fixtureHome, auditDirectory]) {
    writeExclusive(path.join(directory, OWNER_FILE), JSON.stringify(ownership, null, 2));
  }
  process.stderr.write(
    `Owned fixture created; retained for explicit verified cleanup: ${JSON.stringify({ runId, fixtureHome, auditDirectory })}\n`,
  );
  const gameRoot = path.join(fixtureHome, 'Game');
  const root = path.join(gameRoot, 'Mods');
  fs.mkdirSync(root, { recursive: true });
  const objects = Array.from({ length: OBJECT_COUNT }, (_, index) => {
    const name = `Object${String(index).padStart(3, '0')}`;
    const disabled = index % 7 === 0;
    const folder = `${disabled ? 'DISABLED ' : ''}${name}`;
    fs.mkdirSync(path.join(root, folder));
    return { id: `audit-object-${index}`, name, folder, disabled };
  });
  const mods = Array.from({ length: MOD_COUNT }, (_, index) => {
    const object = objects[index % OBJECT_COUNT];
    const name = `Mod${String(index).padStart(5, '0')}`;
    const disabled = index % 13 === 0;
    const folder = `${object.folder}/${disabled ? 'DISABLED ' : ''}${name}`;
    const directory = path.join(root, folder);
    fs.mkdirSync(directory);
    const info = {
      actual_name: name,
      author: 'Synthetic audit fixture',
      description: 'Owned test data',
      version: '1.0',
      tags: ['audit'],
      is_safe: index % 11 !== 0,
      is_favorite: index % 17 === 0,
      is_pinned: index % 19 === 0,
      is_auto_sync: false,
      preset_name: [],
      metadata: {},
    };
    writeExclusive(path.join(directory, 'info.json'), JSON.stringify(info));
    writeExclusive(
      path.join(directory, 'mod.ini'),
      `[Constants]\nglobal $audit_variant = 0\n\n[KeyAuditVariant]\nkey = F8\ntype = cycle\n$audit_variant = 0, 1\n\n[TextureOverrideAudit]\nhash = abcdef12\n`,
    );
    return { id: `audit-mod-${index}`, objectId: object.id, name, folder, disabled, info };
  });
  const db = path.join(auditDirectory, 'library.db');
  const projectionDb = path.join(auditDirectory, 'projection-benchmark.db');
  createDatabase(db, gameRoot, root, objects, mods);
  fs.copyFileSync(db, projectionDb, fs.constants.COPYFILE_EXCL);
  const descriptor = {
    ...ownership,
    ownerFile: OWNER_FILE,
    root,
    db,
    projectionDb,
    counts: {
      objects: OBJECT_COUNT,
      mods: MOD_COUNT,
      disabledObjects: objects.filter((object) => object.disabled).length,
      disabledMods: mods.filter((mod) => mod.disabled).length,
      enabledModsUnderDisabledAncestors: mods.filter(
        (mod, index) => !mod.disabled && objects[index % OBJECT_COUNT].disabled,
      ).length,
    },
    env: {
      EMMM_AUDIT_MODS_ROOT: root,
      EMMM_AUDIT_MODS_PATH: root,
      EMMM_AUDIT_DATABASE_PATH: db,
      EMMM_AUDIT_PROJECTION_DATABASE: projectionDb,
      EMMM_AUDIT_TEST_RUN_ID: runId,
    },
    safety: {
      syntheticOnly: true,
      migrations: 'src-tauri/migrations/*.sql',
      databaseOutsideProtectedGameRoot: true,
      payloadOwned: true,
      autoCleanup: false,
    },
  };
  const descriptorPath = path.join(auditDirectory, 'fixture.json');
  writeExclusive(descriptorPath, JSON.stringify(descriptor, null, 2));
  return { ...descriptor, descriptorPath };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  assert.equal(process.argv.length, 2, 'This script accepts no paths or inherited library inputs');
  const fixture = prepareIgnoredAuditFixture();
  process.stdout.write(
    `${JSON.stringify({ descriptorPath: fixture.descriptorPath, runId: fixture.runId, counts: fixture.counts }, null, 2)}\n`,
  );
}
