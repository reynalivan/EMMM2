import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

const WORKSPACE_ROOT = process.cwd();
const LIB_RS_PATH = 'src-tauri/src/lib.rs';
const PERMISSIONS_PATH = 'src-tauri/permissions/app-commands.toml';
const PERMISSIONS_DIRECTORY = 'src-tauri/permissions';
const BINDINGS_PATH = 'src/lib/bindings.ts';
const COLLECT_COMMANDS_MACRO = 'collect_commands![';
const BACKEND_ONLY_COMMANDS: string[] = [];

type CommandDiff = {
  leftOnly: string[];
  rightOnly: string[];
};

function readWorkspaceFile(relativePath: string): string {
  return readFileSync(join(WORKSPACE_ROOT, relativePath), 'utf8');
}

function uniqueSorted(values: string[]): string[] {
  return Array.from(new Set(values)).sort((left, right) => left.localeCompare(right));
}

function diffCommands(left: string[], right: string[]): CommandDiff {
  const rightSet = new Set(right);
  const leftSet = new Set(left);

  return {
    leftOnly: left.filter((command) => !rightSet.has(command)),
    rightOnly: right.filter((command) => !leftSet.has(command)),
  };
}

function expectNoDuplicates(label: string, values: string[]): void {
  const seen = new Set<string>();
  const duplicates = new Set<string>();

  for (const value of values) {
    if (seen.has(value)) {
      duplicates.add(value);
      continue;
    }

    seen.add(value);
  }

  expect(Array.from(duplicates).sort(), `${label} contains duplicate commands`).toEqual([]);
}

function collectMacroStart(source: string, occurrence: number): number {
  let searchFrom = 0;

  for (let index = 0; index <= occurrence; index += 1) {
    const start = source.indexOf(COLLECT_COMMANDS_MACRO, searchFrom);
    if (start === -1) {
      throw new Error(`Unable to find collect_commands occurrence ${occurrence}`);
    }

    if (index === occurrence) {
      return start + COLLECT_COMMANDS_MACRO.length - 1;
    }

    searchFrom = start + COLLECT_COMMANDS_MACRO.length;
  }

  throw new Error(`Unable to find collect_commands occurrence ${occurrence}`);
}

function extractBracketBody(source: string, openingBracket: number): string {
  let depth = 0;

  for (let index = openingBracket; index < source.length; index += 1) {
    const char = source[index];
    if (char === '[') {
      depth += 1;
      continue;
    }

    if (char !== ']') {
      continue;
    }

    depth -= 1;
    if (depth === 0) {
      return source.slice(openingBracket + 1, index);
    }
  }

  throw new Error('Unable to find collect_commands closing bracket');
}

function parseRustCommandNames(body: string): string[] {
  const commandMatches = body.matchAll(/commands::[a-zA-Z0-9_:]+::([a-zA-Z0-9_]+)/g);
  return uniqueSorted(Array.from(commandMatches, (match) => match[1]));
}

function parseCollectCommands(source: string, occurrence: number): string[] {
  const openingBracket = collectMacroStart(source, occurrence);
  const body = extractBracketBody(source, openingBracket);
  return parseRustCommandNames(body);
}

function parsePermissionCommands(source: string): string[] {
  const declaration = 'commands.allow = [';
  const start = source.indexOf(declaration);
  if (start === -1) {
    throw new Error('Unable to find permissions command allow list');
  }

  const openingBracket = start + declaration.length - 1;
  const body = extractBracketBody(source, openingBracket);
  return uniqueSorted(Array.from(body.matchAll(/"([a-zA-Z0-9_]+)"/g), (match) => match[1]));
}

function parseAllPermissionCommands(): string[] {
  const permissionFiles = readdirSync(join(WORKSPACE_ROOT, PERMISSIONS_DIRECTORY))
    .filter((fileName) => fileName.endsWith('.toml'))
    .map((fileName) => readWorkspaceFile(`${PERMISSIONS_DIRECTORY}/${fileName}`));

  return uniqueSorted(
    permissionFiles.flatMap((source) =>
      Array.from(source.matchAll(/"([a-zA-Z0-9_]+)"/g), (match) => match[1]),
    ),
  );
}

function skipWhitespace(source: string, index: number): number {
  let next = index;
  while (next < source.length && /\s/.test(source[next])) {
    next += 1;
  }

  return next;
}

function skipTypeArguments(source: string, index: number): number {
  if (source[index] !== '<') {
    return index;
  }

  let depth = 0;
  for (let next = index; next < source.length; next += 1) {
    const char = source[next];
    if (char === '<') {
      depth += 1;
      continue;
    }

    if (char !== '>') {
      continue;
    }

    depth -= 1;
    if (depth === 0) {
      return next + 1;
    }
  }

  throw new Error('Unable to parse invoke generic type arguments');
}

function parseBindingInvokeCommands(source: string): string[] {
  const commands: string[] = [];
  let searchFrom = 0;

  while (searchFrom < source.length) {
    const invokeIndex = source.indexOf('invoke', searchFrom);
    if (invokeIndex === -1) {
      break;
    }

    let next = skipWhitespace(source, invokeIndex + 'invoke'.length);
    next = skipTypeArguments(source, next);
    next = skipWhitespace(source, next);

    if (source[next] !== '(') {
      searchFrom = invokeIndex + 'invoke'.length;
      continue;
    }

    next = skipWhitespace(source, next + 1);
    const quote = source[next];
    if (quote !== "'" && quote !== '"') {
      throw new Error('Expected invoke command name as first argument');
    }

    const commandEnd = source.indexOf(quote, next + 1);
    if (commandEnd === -1) {
      throw new Error('Unable to parse invoke command name');
    }

    commands.push(source.slice(next + 1, commandEnd));
    searchFrom = commandEnd + 1;
  }

  return uniqueSorted(commands);
}

function assertSameCommands(label: string, left: string[], right: string[]): void {
  const diff = diffCommands(left, right);

  expect(diff.leftOnly, `${label}: left-only commands`).toEqual([]);
  expect(diff.rightOnly, `${label}: right-only commands`).toEqual([]);
}

describe('command registry audit', () => {
  const libSource = readWorkspaceFile(LIB_RS_PATH);
  const permissionSource = readWorkspaceFile(PERMISSIONS_PATH);
  const bindingSource = readWorkspaceFile(BINDINGS_PATH);
  const productionCommands = parseCollectCommands(libSource, 0);
  const spectaCommands = parseCollectCommands(libSource, 1);
  const permissionCommands = parsePermissionCommands(permissionSource);
  const allPermissionCommands = parseAllPermissionCommands();
  const bindingCommands = parseBindingInvokeCommands(bindingSource);

  it('keeps production, Specta, and permissions command lists aligned', () => {
    expectNoDuplicates('production registry', productionCommands);
    expectNoDuplicates('Specta registry', spectaCommands);
    expectNoDuplicates('permissions registry', permissionCommands);

    assertSameCommands('production vs Specta', productionCommands, spectaCommands);
    assertSameCommands('production vs permissions', productionCommands, permissionCommands);
  });

  it('does not let frontend bindings invoke unregistered commands', () => {
    const productionSet = new Set(productionCommands);
    const missingCommands = bindingCommands.filter((command) => !productionSet.has(command));

    expect(missingCommands).toEqual([]);
  });

  it('does not keep permission-only commands in any permission file', () => {
    const productionSet = new Set(productionCommands);
    const permissionOnlyCommands = allPermissionCommands.filter(
      (command) => !productionSet.has(command),
    );

    expect(permissionOnlyCommands).toEqual([]);
  });

  it('requires production commands to be frontend-bound or explicitly backend-only', () => {
    const bindingSet = new Set(bindingCommands);
    const backendOnlySet = new Set(BACKEND_ONLY_COMMANDS);
    const unclassifiedCommands = productionCommands.filter(
      (command) => !bindingSet.has(command) && !backendOnlySet.has(command),
    );

    expectNoDuplicates('backend-only command policy', BACKEND_ONLY_COMMANDS);
    expect(unclassifiedCommands).toEqual([]);
  });

  it('does not expose removed frontend command aliases', () => {
    const staleAliases = ['get' + 'LogLines', 'get' + 'Game', 'get' + 'WatcherState'];
    const offenders = staleAliases.filter((alias) => bindingSource.includes(`${alias}:`));

    expect(offenders).toEqual([]);
  });

  it('does not keep removed legacy command names in active command registries', () => {
    const staleCommandNames = [
      ['bulk', 'delete', 'mods', 'by', 'ids'].join('_'),
      ['get', 'log', 'lines'].join('_'),
      ['resolve', 'folder', 'collision'].join('_'),
      ['pin', 'object', 'cmd'].join('_'),
      ['get', 'file', 'watcher', 'state'].join('_'),
      ['get', 'game'].join('_'),
      ['check', 'pending', 'tasks'].join('_'),
      ['handle', 'dirty', 'state'].join('_'),
      ['handle', 'mod', 'moved', 'or', 'renamed'].join('_'),
      ['create', 'download', 'session'].join('_'),
      ['get', 'hotkey', 'bindings'].join('_'),
      ['detect', 'hotkey', 'conflicts'].join('_'),
      ['pin', 'mod'].join('_'),
      ['repair', 'orphan', 'mods'].join('_'),
    ];
    const activeCommandNames = uniqueSorted([
      ...productionCommands,
      ...spectaCommands,
      ...permissionCommands,
      ...bindingCommands,
    ]);
    const offenders = staleCommandNames.filter((command) => activeCommandNames.includes(command));

    expect(offenders).toEqual([]);
  });
});
