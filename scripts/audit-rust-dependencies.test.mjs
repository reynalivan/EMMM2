import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  auditFindings,
  evaluateAudit,
  productionGraph,
  RELEASE_TARGET,
  runAudit,
  sourceTreeDigest,
} from './audit-rust-dependencies.mjs';

function isolatedRepo(t) {
  const root = mkdtempSync(join(tmpdir(), 'emmm-advisory-policy-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, 'src-tauri/src'), { recursive: true });
  mkdirSync(join(root, 'security'));
  writeFileSync(join(root, 'src-tauri/build.rs'), 'fn main() {}\n');
  writeFileSync(join(root, 'src-tauri/src/lib.rs'), 'pub fn safe() {}\n');
  const input = fixture();
  input.policy.exceptions[0].reviewedAt = new Date().toISOString();
  input.policy.exceptions[0].expiresAt = new Date(Date.now() + 86400000).toISOString();
  writeFileSync(join(root, 'security/rustsec-exceptions.json'), JSON.stringify(input.policy));
  return { root, input, output: join(root, 'output') };
}

test('source digest normalizes CRLF but rejects new or changed Rust code', (t) => {
  const { root } = isolatedRepo(t);
  const before = sourceTreeDigest(root);
  writeFileSync(join(root, 'src-tauri/src/lib.rs'), 'pub fn safe() {}\r\n');
  assert.equal(sourceTreeDigest(root), before);
  writeFileSync(join(root, 'src-tauri/src/new.rs'), 'pub fn new_sink() {}\n');
  assert.notEqual(sourceTreeDigest(root), before);
  rmSync(join(root, 'src-tauri/src/new.rs'));
  writeFileSync(join(root, 'src-tauri/build.rs'), 'fn main() { panic!(); }\n');
  assert.notEqual(sourceTreeDigest(root), before);
});

test('command integration retains unfiltered JSON, stderr, selected tree and decisions', (t) => {
  const { root, input, output } = isolatedRepo(t);
  const execute = (command, args, options) => {
    assert.equal(command, 'cargo');
    assert.equal(options.cwd, join(root, 'src-tauri'));
    if (args[0] === 'audit') {
      assert.deepEqual(args, ['audit', '--json', '--deny', 'warnings']);
      return { status: 1, stdout: JSON.stringify(input.report), stderr: 'audit diagnostics' };
    }
    assert.deepEqual(args, [
      'tree',
      '--locked',
      '--target',
      RELEASE_TARGET,
      '-e',
      'normal,build',
      '--no-dedupe',
      '--prefix',
      'depth',
      '--charset',
      'ascii',
      '--format',
      '{p}|{f}',
    ]);
    return { status: 0, stdout: input.tree, stderr: '' };
  };
  runAudit(root, output, execute);
  assert.deepEqual(
    JSON.parse(readFileSync(join(output, 'cargo-audit.json'), 'utf8')),
    input.report,
  );
  assert.equal(readFileSync(join(output, 'cargo-audit.stderr.log'), 'utf8'), 'audit diagnostics');
  assert.equal(readFileSync(join(output, 'windows-tree.txt'), 'utf8'), input.tree);
  assert.match(readFileSync(join(output, 'decisions.txt'), 'utf8'), /feature-not-enabled/);
});

for (const [name, result, message] of [
  ['missing executable', { error: new Error('ENOENT'), status: null }, /could not run/],
  ['tool failure', { status: 2 }, /could not run/],
  ['invalid JSON', { status: 1, stdout: '{' }, /JSON/],
  ['inconsistent successful exit', { status: 0 }, /exit status/],
]) {
  test(`command integration fails closed: ${name}`, (t) => {
    const { root, input, output } = isolatedRepo(t);
    assert.throws(
      () =>
        runAudit(root, output, () => ({
          stdout: JSON.stringify(input.report),
          stderr: 'retained error',
          ...result,
        })),
      message,
    );
    assert.equal(readFileSync(join(output, 'cargo-audit.stderr.log'), 'utf8'), 'retained error');
  });
}

test('failed Cargo tree blocks policy evaluation even if its stdout looks valid', (t) => {
  const { root, input, output } = isolatedRepo(t);
  assert.throws(
    () =>
      runAudit(root, output, (_command, args) =>
        args[0] === 'audit'
          ? { status: 1, stdout: JSON.stringify(input.report), stderr: '' }
          : { status: 1, stdout: input.tree, stderr: 'tree error' },
      ),
    /Cargo tree failed/,
  );
  assert.equal(readFileSync(join(output, 'windows-tree.stderr.log'), 'utf8'), 'tree error');
});

function fixture() {
  const finding = {
    advisory: { id: 'RUSTSEC-2026-0097' },
    package: { name: 'rand', version: '0.7.3' },
  };
  return {
    now: new Date('2026-10-19T00:00:00Z'),
    report: {
      settings: {
        ignore: [],
        severity: null,
        target_arch: [],
        target_os: [],
        informational_warnings: ['unmaintained', 'unsound', 'notice'],
      },
      database: { 'last-commit': 'test-commit' },
      vulnerabilities: { count: 0, list: [] },
      warnings: { unsound: [finding] },
    },
    tree: '0emmm v2.0.1 (/repo)|\n1phf_generator v0.8.0|\n2rand v0.7.3|std\n',
    sourceDigest: 'a'.repeat(64),
    policy: {
      schemaVersion: 1,
      target: RELEASE_TARGET,
      exceptions: [
        {
          advisory: 'RUSTSEC-2026-0097',
          package: 'rand',
          version: '0.7.3',
          kind: 'unsound',
          disposition: 'feature-not-enabled',
          forbiddenFeature: 'log',
          owner: 'maintainer',
          reviewedAt: '2026-10-18T00:00:00Z',
          expiresAt: '2026-11-17T00:00:00Z',
          reason: 'The necessary log feature is disabled.',
          evidence: 'Reviewed Cargo tree.',
          followUp: 'Replace PHF dependency.',
          graph: { features: ['std'], parents: ['phf_generator@0.8.0'] },
        },
      ],
    },
  };
}

test('accepts an exact, owned, unexpired feature exception without hiding raw findings', () => {
  const input = fixture();
  const before = structuredClone(input.report);
  assert.equal(evaluateAudit(input).length, 1);
  assert.deepEqual(input.report, before);
  assert.equal(auditFindings(input.report).length, 1);
});

test('collects repeated parents and unions host/build feature contexts', () => {
  const graph = productionGraph(
    '0emmm v2.0.1 (/repo)|\n1a v1.0.0|\n2rand v0.7.3|std\n1b v1.0.0|\n2rand v0.7.3|log,std\n',
  );
  assert.deepEqual(graph.get('rand@0.7.3'), {
    features: ['log', 'std'],
    parents: ['a@1.0.0', 'b@1.0.0'],
  });
});

for (const [name, mutate, error] of [
  [
    'expired review',
    (i) => {
      i.now = new Date('2026-11-17T00:00:00Z');
    },
    /Expired/,
  ],
  [
    'future review',
    (i) => {
      i.now = new Date('2026-10-17T00:00:00Z');
    },
    /invalid review/,
  ],
  [
    'overlong exception',
    (i) => {
      i.policy.exceptions[0].expiresAt = '2026-11-18T00:00:00Z';
    },
    /review window/,
  ],
  [
    'missing owner',
    (i) => {
      delete i.policy.exceptions[0].owner;
    },
    /Incomplete exception/,
  ],
  [
    'missing follow-up',
    (i) => {
      delete i.policy.exceptions[0].followUp;
    },
    /Incomplete exception/,
  ],
  [
    'changed target',
    (i) => {
      i.target = 'aarch64-apple-darwin';
    },
    /Unreviewed release target/,
  ],
  [
    'changed policy target',
    (i) => {
      i.policy.target = 'aarch64-apple-darwin';
    },
    /Unreviewed release target/,
  ],
  [
    'enabled log',
    (i) => {
      i.tree = i.tree.replace('|std', '|log,std');
    },
    /Features or parents changed/,
  ],
  [
    'enabled log despite renewed graph',
    (i) => {
      i.tree = i.tree.replace('|std', '|log,std');
      i.policy.exceptions[0].graph.features.unshift('log');
    },
    /Vulnerable feature enabled/,
  ],
  [
    'changed parent',
    (i) => {
      i.tree = i.tree.replace('phf_generator', 'another');
    },
    /Features or parents changed/,
  ],
  [
    'changed version',
    (i) => {
      i.report.warnings.unsound[0].package.version = '0.7.4';
    },
    /Unreviewed finding/,
  ],
  [
    'new advisory',
    (i) => {
      i.report.warnings.unsound[0].advisory.id = 'RUSTSEC-2026-9999';
    },
    /Unreviewed finding/,
  ],
  [
    'unreported advisory',
    (i) => {
      i.report.warnings = {};
    },
    /Stale exception/,
  ],
  [
    'globally ignored advisory',
    (i) => {
      i.report.settings.ignore.push('RUSTSEC-2026-0097');
    },
    /unfiltered/,
  ],
  [
    'severity filter',
    (i) => {
      i.report.settings.severity = 'high';
    },
    /unfiltered/,
  ],
  [
    'warning filter',
    (i) => {
      i.report.settings.informational_warnings = ['unsound'];
    },
    /unfiltered/,
  ],
  [
    'missing database',
    (i) => {
      delete i.report.database;
    },
    /Incomplete audit/,
  ],
  [
    'missing vulnerability records',
    (i) => {
      i.report.vulnerabilities.count = 1;
    },
    /Incomplete audit/,
  ],
  [
    'duplicate exception',
    (i) => {
      i.policy.exceptions.push(structuredClone(i.policy.exceptions[0]));
    },
    /Duplicate exception/,
  ],
  [
    'duplicate finding',
    (i) => {
      i.report.warnings.unsound.push(structuredClone(i.report.warnings.unsound[0]));
    },
    /Duplicate finding/,
  ],
  [
    'misclassified vulnerability',
    (i) => {
      i.policy.exceptions[0].disposition = 'maintenance-risk-accepted';
    },
    /Cannot accept/,
  ],
]) {
  test(`fails closed: ${name}`, () => {
    const input = fixture();
    mutate(input);
    assert.throws(() => evaluateAudit(input), error);
  });
}

test('absence exception fails as soon as the package enters the production graph', () => {
  const input = fixture();
  Object.assign(input.policy.exceptions[0], { disposition: 'not-in-windows-graph', graph: null });
  const original = input.tree;
  input.tree = '0emmm v2.0.1 (/repo)|\n';
  assert.equal(evaluateAudit(input).length, 1);
  input.tree = original;
  assert.throws(() => evaluateAudit(input), /Package now reachable/);
});

test('source reachability exception requires the reviewed whole-tree digest', () => {
  const input = fixture();
  Object.assign(input.policy.exceptions[0], {
    disposition: 'vulnerable-code-not-used',
    sourceTreeSha256: input.sourceDigest,
  });
  assert.equal(evaluateAudit(input).length, 1);
  input.sourceDigest = 'b'.repeat(64);
  assert.throws(() => evaluateAudit(input), /Source changed/);
});

test('unknown or malformed warning records and Cargo output block the gate', () => {
  const input = fixture();
  input.report.warnings.yanked = [{ package: { name: 'other', version: '1.0.0' } }];
  assert.throws(() => evaluateAudit(input), /Unreviewed finding/);
  for (const tree of [
    '',
    '0other v1.0.0|',
    '0emmm v1.0.0|\n2rand v0.7.3|',
    '0emmm v1.0.0|\n0emmm v1.0.0|',
    '0emmm v1.0.0|\n1rand v0.7.3 (*)|std',
  ]) {
    assert.throws(() => productionGraph(tree), /Cargo/);
  }
});
