import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const workspaceRoot = resolve(import.meta.dirname, '..');
const packagePath = resolve(workspaceRoot, 'package.json');
const cargoPath = resolve(workspaceRoot, 'src-tauri', 'Cargo.toml');
const tauriConfigPath = resolve(workspaceRoot, 'src-tauri', 'tauri.conf.json');
const versionPattern =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/;

function usage(message) {
  if (message) {
    console.error(`Error: ${message}`);
  }
  console.error('Usage: pnpm release:prepare -- --version <semver> [--create-tag]');
  process.exitCode = 1;
}

function parseArguments(argv) {
  let version;
  let createTag = false;

  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === '--') {
      continue;
    }
    if (argument === '--version') {
      version = argv[index + 1];
      index += 1;
      continue;
    }
    if (argument === '--create-tag') {
      createTag = true;
      continue;
    }
    usage(`Unknown argument '${argument}'.`);
    return null;
  }

  if (!version || !versionPattern.test(version)) {
    usage('Use a SemVer version without a leading v, for example 0.1.0.');
    return null;
  }
  return { version, createTag };
}

function replaceExactlyOnce(source, pattern, replacement, filePath) {
  const firstMatch = source.match(pattern);
  const hasSecondMatch =
    firstMatch?.index !== undefined &&
    source.slice(firstMatch.index + firstMatch[0].length).match(pattern);
  if (!firstMatch || hasSecondMatch) {
    throw new Error(`Could not update the release version in ${filePath}.`);
  }
  return source.replace(pattern, replacement);
}

function updateVersions(version) {
  const packageJson = JSON.parse(readFileSync(packagePath, 'utf8'));
  packageJson.version = version;
  writeFileSync(packagePath, `${JSON.stringify(packageJson, null, 2)}\n`);

  const cargoSource = readFileSync(cargoPath, 'utf8');
  const cargoUpdated = replaceExactlyOnce(
    cargoSource,
    /^(version\s*=\s*)"[^"]+"/m,
    `$1"${version}"`,
    cargoPath,
  );
  writeFileSync(cargoPath, cargoUpdated);

  const tauriSource = readFileSync(tauriConfigPath, 'utf8');
  const tauriUpdated = replaceExactlyOnce(
    tauriSource,
    /^(\s*"version"\s*:\s*)"[^"]+"/m,
    `$1"${version}"`,
    tauriConfigPath,
  );
  writeFileSync(tauriConfigPath, tauriUpdated);
}

function readConfiguration() {
  const packageJson = JSON.parse(readFileSync(packagePath, 'utf8'));
  const cargoSource = readFileSync(cargoPath, 'utf8');
  const tauriConfig = JSON.parse(readFileSync(tauriConfigPath, 'utf8'));
  const cargoVersion = cargoSource.match(/^version\s*=\s*"([^"]+)"/m)?.[1];

  return { packageJson, cargoVersion, tauriConfig };
}

function validateReleaseConfiguration(version) {
  const { packageJson, cargoVersion, tauriConfig } = readConfiguration();
  const publicKey = tauriConfig.plugins?.updater?.pubkey?.trim();
  const endpoints = tauriConfig.plugins?.updater?.endpoints;

  const configuredVersions = [packageJson.version, cargoVersion, tauriConfig.version];
  if (configuredVersions.some((configuredVersion) => configuredVersion !== version)) {
    throw new Error(`Release versions are not synchronized: ${configuredVersions.join(', ')}.`);
  }
  if (!publicKey || publicKey.includes('REPLACE_WITH_')) {
    throw new Error('Tauri updater public key is missing.');
  }
  if (
    !Array.isArray(endpoints) ||
    endpoints.length === 0 ||
    endpoints.some((endpoint) => !endpoint.startsWith('https://'))
  ) {
    throw new Error('Tauri updater requires at least one HTTPS endpoint.');
  }
  if (tauriConfig.bundle?.createUpdaterArtifacts !== true) {
    throw new Error('bundle.createUpdaterArtifacts must be true.');
  }
}

function gitOutput(args) {
  return execFileSync('git', args, { cwd: workspaceRoot, encoding: 'utf8' }).trim();
}

function createLocalTag(version) {
  if (gitOutput(['status', '--porcelain'])) {
    throw new Error(
      'Refusing to create a tag with a dirty working tree. Commit the intended release first.',
    );
  }

  const tag = `v${version}`;
  if (gitOutput(['tag', '--list', tag])) {
    throw new Error(`Git tag ${tag} already exists.`);
  }

  execFileSync('git', ['tag', '-a', tag, '-m', `Release ${tag}`], {
    cwd: workspaceRoot,
    stdio: 'inherit',
  });
  console.log(`Created local tag ${tag}. No push was performed.`);
}

const options = parseArguments(process.argv.slice(2));
if (options) {
  try {
    updateVersions(options.version);
    validateReleaseConfiguration(options.version);
    console.log(`Release ${options.version} is configured for a signed updater.`);
    if (options.createTag) {
      createLocalTag(options.version);
    } else {
      console.log(`No tag or push was performed. Create a local tag later with --create-tag.`);
    }
  } catch (error) {
    console.error(
      error instanceof Error ? `Error: ${error.message}` : 'Error: release preparation failed.',
    );
    process.exitCode = 1;
  }
}
