import fs from 'node:fs';
import path from 'node:path';
import { DatabaseSync, backup as backupDatabase } from 'node:sqlite';
import { execFileSync, spawn } from 'node:child_process';
import { randomUUID, createHash } from 'node:crypto';

export const READONLY_AUDIT_IDENTIFIER = 'com.reynalivan.emmm.e2e.readonly-audit';
export const READONLY_AUDIT_GAME = 'gimi-readonly-startup-audit';
const BACKUP_FILES = ['app.db', 'app.db-wal', 'app.db-shm', 'mutation-journal.json'] as const;

export function readonlyAuditPaths() {
  const root = process.env.EMMM_READ_ONLY_AUDIT_ROOT;
  const output = process.env.EMMM_READ_ONLY_AUDIT_OUTPUT;
  const appDataParent = process.env.APPDATA;
  if (!root || !output || !appDataParent) {
    throw new Error('Read-only audit root, output and APPDATA are required');
  }
  const realRoot = fs.realpathSync(root);
  const mods = fs.realpathSync(path.join(realRoot, 'Mods'));
  if (path.dirname(mods) !== realRoot)
    throw new Error('Mods must be a physical child of the protected root');
  const appData = canonicalPlannedPath(path.join(appDataParent, READONLY_AUDIT_IDENTIFIER));
  const auditOutput = canonicalPlannedPath(output);
  for (const candidate of [appData, auditOutput]) {
    const relative = path.relative(realRoot, candidate);
    const reverse = path.relative(candidate, realRoot);
    const outside = (value: string) =>
      value === '..' || value.startsWith(`..${path.sep}`) || path.isAbsolute(value);
    if (!outside(relative) || !outside(reverse)) {
      throw new Error('Audit config/output must be outside the real library');
    }
  }
  if (path.basename(appData) !== READONLY_AUDIT_IDENTIFIER) {
    throw new Error('Audit app data must resolve to the isolated E2E identifier');
  }
  if (fs.existsSync(appData)) assertNoLinks(appData);
  const summary = JSON.parse(
    fs.readFileSync(path.join(auditOutput, 'before-summary.json'), 'utf8'),
  ) as unknown;
  if (
    typeof summary !== 'object' ||
    summary === null ||
    !('root' in summary) ||
    typeof summary.root !== 'string' ||
    fs.realpathSync(summary.root) !== realRoot
  ) {
    throw new Error('Completed checksum baseline for this exact root is required');
  }
  const beforeManifest = fs.readFileSync(path.join(auditOutput, 'before-manifest.json'));
  if (
    !('phase' in summary) ||
    summary.phase !== 'Before' ||
    !('metadataSource' in summary) ||
    summary.metadataSource !== 'WindowsHandleV2' ||
    !('manifestSha256' in summary) ||
    typeof summary.manifestSha256 !== 'string' ||
    createHash('sha256').update(beforeManifest).digest('hex').toUpperCase() !==
      summary.manifestSha256
  ) {
    throw new Error('Audit checksum baseline is incomplete or inconsistent');
  }
  return { root: realRoot, mods, appData, output: auditOutput };
}

export async function prepareReadonlyAudit() {
  const audit = readonlyAuditPaths();
  const appData = audit.appData;
  ensureNativeAuditStopped();
  ensureInstalledAppStopped();
  if (!fs.existsSync(appData)) {
    fs.mkdirSync(appData);
    const template = path.join(path.dirname(appData), 'com.reynalivan.emmm.e2e', 'app.db');
    const source = new DatabaseSync(template, { readOnly: true });
    try {
      await backupDatabase(source, path.join(appData, 'app.db'));
    } finally {
      source.close();
    }
    const isolated = new DatabaseSync(path.join(appData, 'app.db'));
    try {
      isolated.exec('PRAGMA foreign_keys = ON; DELETE FROM games');
      isolated.prepare('DELETE FROM app_settings WHERE key = ?').run('active_game_id');
      isolated.exec('PRAGMA wal_checkpoint(TRUNCATE)');
    } finally {
      isolated.close();
    }
  }
  const backup = path.join(audit.output, 'isolated-config-backup');
  if (fs.existsSync(backup))
    throw new Error('Existing audit backup must be recovered before another run');
  fs.mkdirSync(backup, { recursive: true });
  const present: string[] = [];
  for (const name of BACKUP_FILES) {
    const source = path.join(audit.appData, name);
    if (fs.existsSync(source)) {
      fs.copyFileSync(source, path.join(backup, name), fs.constants.COPYFILE_EXCL);
      present.push(name);
    }
  }
  fs.writeFileSync(path.join(backup, 'files.json'), JSON.stringify(present));
  const db = new DatabaseSync(path.join(audit.appData, 'app.db'));
  try {
    db.exec('PRAGMA foreign_keys = ON');
    const row = db.prepare('SELECT value FROM app_settings WHERE key = ?').get('hotkeys');
    if (typeof row?.value !== 'string')
      throw new Error('Seed requires migrated isolated hotkey settings');
    const hotkeys = JSON.parse(row.value) as unknown;
    if (typeof hotkeys !== 'object' || hotkeys === null)
      throw new Error('Invalid isolated hotkey settings');
    db.exec('BEGIN IMMEDIATE');
    db.exec('DELETE FROM games');
    db.prepare(
      'INSERT INTO games (id, name, game_type, path, mods_path) VALUES (?, ?, ?, ?, ?)',
    ).run(READONLY_AUDIT_GAME, 'GIMI Read-only Audit', 0, audit.root, audit.mods);
    const setting = db.prepare(
      'INSERT INTO app_settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value=excluded.value',
    );
    setting.run('active_game_id', READONLY_AUDIT_GAME);
    setting.run('language', 'en');
    setting.run('hotkeys', JSON.stringify({ ...hotkeys, enabled: false }));
    setting.run('diagnostics', JSON.stringify({ telemetry_enabled: false }));
    db.exec('COMMIT');
    db.exec('PRAGMA wal_checkpoint(TRUNCATE)');
  } finally {
    db.close();
  }
  const journal = path.join(audit.appData, 'mutation-journal.json');
  if (fs.existsSync(journal))
    fs.renameSync(journal, path.join(audit.appData, `audit-detached-journal-${randomUUID()}.json`));
  return audit;
}

export function restoreReadonlyAudit() {
  const audit = readonlyAuditPaths();
  const backup = path.join(audit.output, 'isolated-config-backup');
  if (!fs.existsSync(backup)) return;
  ensureNativeAuditStopped();
  const present: unknown = JSON.parse(fs.readFileSync(path.join(backup, 'files.json'), 'utf8'));
  if (
    !Array.isArray(present) ||
    !present.every(
      (name) =>
        typeof name === 'string' && BACKUP_FILES.includes(name as (typeof BACKUP_FILES)[number]),
    )
  ) {
    throw new Error('Invalid isolated config backup inventory');
  }
  const evidence = path.join(audit.appData, 'audit-config-evidence', randomUUID());
  fs.mkdirSync(evidence, { recursive: true });
  for (const name of BACKUP_FILES) {
    const target = path.join(audit.appData, name);
    if (fs.existsSync(target)) {
      fs.renameSync(target, path.join(evidence, name));
    }
    if (present.includes(name)) fs.copyFileSync(path.join(backup, name), target);
  }
  const db = new DatabaseSync(path.join(audit.appData, 'app.db'), { readOnly: true });
  try {
    const rootBinding = db
      .prepare('SELECT COUNT(*) AS count FROM games WHERE path = ? OR mods_path = ?')
      .get(audit.root, audit.mods);
    if (rootBinding?.count !== 0)
      throw new Error('Restored configuration still points at the audited library');
  } finally {
    db.close();
  }
  fs.renameSync(backup, path.join(audit.output, `restored-config-backup-${randomUUID()}`));
  fs.writeFileSync(
    path.join(audit.output, 'config-restored.json'),
    JSON.stringify({ restored: true, realLibraryBindings: 0 }),
  );
}

function ensureNativeAuditStopped() {
  const count = execFileSync(
    'powershell',
    [
      '-NoProfile',
      '-Command',
      '(Get-Process -Name emmm-readonly-audit -ErrorAction SilentlyContinue | Measure-Object).Count',
    ],
    { encoding: 'utf8', windowsHide: true },
  ).trim();
  if (count !== '0')
    throw new Error('Isolated config cannot be changed while a native E2E process is alive');
}

export function ensureInstalledAppStopped() {
  const count = execFileSync(
    'powershell',
    [
      '-NoProfile',
      '-Command',
      '(Get-Process -Name emmm -ErrorAction SilentlyContinue | Measure-Object).Count',
    ],
    { encoding: 'utf8', windowsHide: true },
  ).trim();
  if (count !== '0')
    throw new Error('Close the installed/development EMMM app before protected native audit');
}

export function monitorInstalledApp(onFailure: (message: string) => void): () => void {
  ensureInstalledAppStopped();
  let stopping = false;
  const child = spawn(
    'powershell',
    [
      '-NoProfile',
      '-Command',
      '$ErrorActionPreference = "Stop"; while ($true) { if (Get-Process -Name emmm -ErrorAction SilentlyContinue) { exit 42 }; Start-Sleep -Milliseconds 500 }',
    ],
    { stdio: 'ignore', windowsHide: true },
  );
  child.on('error', (error) => {
    if (!stopping) onFailure(`Installed-app interlock failed: ${error.message}`);
  });
  child.on('exit', (code) => {
    if (!stopping)
      onFailure(
        code === 42
          ? 'Installed/development EMMM reopened during protected audit'
          : 'Installed-app interlock stopped unexpectedly',
      );
  });
  return () => {
    stopping = true;
    if (child.exitCode === null && child.signalCode === null) child.kill();
  };
}

function canonicalPlannedPath(input: string) {
  let current = path.resolve(input);
  const missing: string[] = [];
  while (!fs.existsSync(current)) {
    missing.unshift(path.basename(current));
    const parent = path.dirname(current);
    if (parent === current) throw new Error('Audit path has no existing ancestor');
    current = parent;
  }
  for (let ancestor = current; ; ancestor = path.dirname(ancestor)) {
    if (fs.lstatSync(ancestor).isSymbolicLink()) throw new Error('Linked audit paths are unsafe');
    if (path.dirname(ancestor) === ancestor) break;
  }
  return path.join(fs.realpathSync(current), ...missing);
}

function assertNoLinks(root: string) {
  const pending = [root];
  while (pending.length > 0) {
    const directory = pending.pop();
    if (!directory) throw new Error('Missing isolated directory');
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const candidate = path.join(directory, entry.name);
      if (entry.isSymbolicLink() || fs.lstatSync(candidate).isSymbolicLink()) {
        throw new Error('Read-only audit refuses linked files/directories in isolated app data');
      }
      if (entry.isDirectory()) pending.push(candidate);
    }
  }
}
