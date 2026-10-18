import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

export const RELEASE_TARGET = 'x86_64-pc-windows-msvc';
const MAX_REVIEW_MS = 30 * 24 * 60 * 60 * 1000;
const fail = (message) => {
  throw new Error(message);
};
const digest = (value) => createHash('sha256').update(value).digest('hex');
const same = (left, right) => JSON.stringify(left) === JSON.stringify(right);

export function sourceTreeDigest(root) {
  const files = ['src-tauri/build.rs'];
  const walk = (directory) => {
    for (const entry of readdirSync(join(root, directory), { withFileTypes: true })) {
      const path = `${directory}/${entry.name}`;
      if (entry.isSymbolicLink()) fail(`Source symlink requires review: ${path}`);
      if (entry.isDirectory()) walk(path);
      else if (entry.name.endsWith('.rs')) files.push(path);
    }
  };
  walk('src-tauri/src');
  return digest(
    files
      .sort()
      .map(
        (file) =>
          `${file}:${digest(readFileSync(join(root, file), 'utf8').replaceAll('\r\n', '\n'))}\n`,
      )
      .join(''),
  );
}

export function productionGraph(tree) {
  // cargo metadata includes optional lockfile-only SQL drivers. cargo tree selects
  // the actual normal/build graph; --no-dedupe retains every incoming edge.
  const result = new Map();
  const stack = [];
  for (const line of tree.trim().split(/\r?\n/)) {
    const match = /^(\d+)([a-zA-Z0-9_-]+) v([^ |]+)([^|]*)\|([^|]*)$/.exec(line);
    if (!match) fail(`Unrecognized Cargo tree output: ${line}`);
    const [, depthText, name, version, source, features] = match;
    if (source.includes('(*)')) fail('Deduplicated Cargo output cannot prove all parents');
    const depth = Number(depthText);
    if ((depth === 0 && (result.size || name !== 'emmm')) || depth > stack.length)
      fail('Incomplete Cargo tree');
    if (depth > 0 && !stack.length) fail('Missing Cargo root');
    const key = `${name}@${version}`;
    const item = result.get(key) ?? { source, features: new Set(), parents: new Set() };
    if (item.source !== source) fail(`Ambiguous package source: ${key}`);
    for (const feature of features.split(',').filter(Boolean)) item.features.add(feature);
    if (depth > 0) item.parents.add(depth === 1 ? 'emmm' : stack[depth - 1]);
    result.set(key, item);
    stack.length = depth;
    stack.push(key);
  }
  return new Map(
    [...result].map(([key, item]) => [
      key,
      {
        features: [...item.features].sort(),
        parents: [...item.parents].sort(),
      },
    ]),
  );
}

export function auditFindings(report) {
  const settings = report.settings;
  if (
    !settings ||
    !same(settings.ignore, []) ||
    settings.severity !== null ||
    !same(settings.target_arch, []) ||
    !same(settings.target_os, []) ||
    !['unmaintained', 'unsound', 'notice'].every((kind) =>
      settings.informational_warnings?.includes(kind),
    )
  ) {
    fail('Audit must be unfiltered, including informational warnings');
  }
  if (
    !report.database?.['last-commit'] ||
    !Array.isArray(report.vulnerabilities?.list) ||
    report.vulnerabilities.count !== report.vulnerabilities.list.length ||
    typeof report.warnings !== 'object' ||
    report.warnings === null
  )
    fail('Incomplete audit report');
  const findings = report.vulnerabilities.list.map((item) => ({ ...item, kind: 'vulnerability' }));
  for (const [kind, entries] of Object.entries(report.warnings)) {
    if (!Array.isArray(entries)) fail(`Invalid audit warning: ${kind}`);
    for (const entry of entries) findings.push({ ...entry, kind });
  }
  return findings;
}

export function evaluateAudit({
  report,
  tree,
  policy,
  sourceDigest,
  now = new Date(),
  target = RELEASE_TARGET,
}) {
  if (target !== RELEASE_TARGET || policy.target !== RELEASE_TARGET || policy.schemaVersion !== 1)
    fail('Unreviewed release target or policy schema');
  if (!Array.isArray(policy.exceptions)) fail('Missing exception inventory');
  const graph = productionGraph(tree);
  const findings = auditFindings(report);
  const rules = new Map();
  for (const rule of policy.exceptions) {
    const key = `${rule.advisory}/${rule.package}@${rule.version}/${rule.kind}`;
    if (rules.has(key)) fail(`Duplicate exception: ${key}`);
    if (
      !/^RUSTSEC-\d{4}-\d{4}$/.test(rule.advisory) ||
      ![rule.package, rule.version, rule.owner, rule.reason, rule.evidence, rule.followUp].every(
        (text) => typeof text === 'string' && text.trim(),
      )
    )
      fail(`Incomplete exception: ${key}`);
    const approved = Date.parse(rule.reviewedAt);
    const expires = Date.parse(rule.expiresAt);
    if (
      ![rule.reviewedAt, rule.expiresAt].every(
        (date) => typeof date === 'string' && date.endsWith('Z'),
      ) ||
      !Number.isFinite(approved) ||
      !Number.isFinite(expires) ||
      approved > now.getTime() ||
      expires <= now.getTime() ||
      expires <= approved ||
      expires - approved > MAX_REVIEW_MS
    )
      fail(`Expired or invalid review window: ${key}`);
    const actual = graph.get(`${rule.package}@${rule.version}`);
    if (rule.disposition === 'not-in-windows-graph') {
      if (actual || !same(rule.graph, null)) fail(`Package now reachable: ${key}`);
    } else if (
      ['feature-not-enabled', 'vulnerable-code-not-used', 'maintenance-risk-accepted'].includes(
        rule.disposition,
      )
    ) {
      if (!actual || !same(actual, rule.graph)) fail(`Features or parents changed: ${key}`);
      if (rule.disposition === 'maintenance-risk-accepted' && rule.kind !== 'unmaintained')
        fail(`Cannot accept a vulnerability as maintenance risk: ${key}`);
      if (
        rule.disposition === 'feature-not-enabled' &&
        (!rule.forbiddenFeature || actual.features.includes(rule.forbiddenFeature))
      )
        fail(`Vulnerable feature enabled: ${key}`);
      if (
        rule.disposition === 'vulnerable-code-not-used' &&
        (!/^[a-f0-9]{64}$/.test(rule.sourceTreeSha256) || rule.sourceTreeSha256 !== sourceDigest)
      )
        fail(`Source changed; renew reachability review: ${key}`);
    } else fail(`Unknown disposition: ${key}`);
    rules.set(key, rule);
  }
  const used = new Set();
  const decisions = findings.map((finding) => {
    const key = `${finding.advisory?.id}/${finding.package?.name}@${finding.package?.version}/${finding.kind}`;
    const rule = rules.get(key);
    if (!rule) fail(`Unreviewed finding: ${key}`);
    if (used.has(key)) fail(`Duplicate finding: ${key}`);
    used.add(key);
    return `${key}: ${rule.disposition} (owner ${rule.owner}, review before ${rule.expiresAt})`;
  });
  for (const key of rules.keys())
    if (!used.has(key)) fail(`Stale exception; remove or review: ${key}`);
  return decisions;
}

export function runAudit(root, outputDirectory, execute = spawnSync) {
  mkdirSync(outputDirectory, { recursive: true });
  const run = (name, args) => {
    const result = execute('cargo', args, {
      cwd: join(root, 'src-tauri'),
      encoding: 'utf8',
      maxBuffer: 64 * 1024 * 1024,
    });
    writeFileSync(
      join(outputDirectory, `${name}.${name === 'cargo-audit' ? 'json' : 'txt'}`),
      result.stdout ?? '',
    );
    writeFileSync(join(outputDirectory, `${name}.stderr.log`), result.stderr ?? '');
    if (result.error || result.signal || ![0, 1].includes(result.status))
      fail(`${name} could not run: ${result.error?.message ?? result.signal ?? result.status}`);
    return { stdout: result.stdout, status: result.status };
  };
  const audit = run('cargo-audit', ['audit', '--json', '--deny', 'warnings']);
  const report = JSON.parse(audit.stdout);
  const findings = auditFindings(report);
  if (audit.status !== (findings.length ? 1 : 0)) fail('Audit exit status disagrees with report');
  const tree = run('windows-tree', [
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
  if (tree.status !== 0) fail('Cargo tree failed');
  const decisions = evaluateAudit({
    report,
    tree: tree.stdout,
    policy: JSON.parse(readFileSync(join(root, 'security/rustsec-exceptions.json'), 'utf8')),
    sourceDigest: sourceTreeDigest(root),
  });
  writeFileSync(join(outputDirectory, 'decisions.txt'), `${decisions.join('\n')}\n`);
  console.log(
    `Full audit retained: ${findings.length} findings; ${decisions.length} bounded exceptions validated for ${RELEASE_TARGET}.`,
  );
  console.log(decisions.join('\n'));
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
  const outputDirectory = process.argv[2];
  if (!outputDirectory || relative(root, resolve(outputDirectory)) === '')
    fail('Provide a dedicated audit output directory');
  try {
    runAudit(root, resolve(outputDirectory));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
