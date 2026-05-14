import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const WORKSPACE_ROOT = process.cwd();
const PERMISSIONS_DIRECTORY = 'src-tauri/permissions';
const BINDINGS_PATH = 'src/lib/bindings.ts';
const COLLECT_COMMANDS_MACRO = 'collect_commands![';
const SOURCE_DIRECTORIES = ['src'];
const EXCLUDED_SOURCE_DIRECTORIES = new Set(['node_modules', 'dist', 'coverage']);

export type CommandDiff = {
  leftOnly: string[];
  rightOnly: string[];
};

export type SourceFile = {
  relativePath: string;
  source: string;
};

export function readWorkspaceFile(relativePath: string): string {
  return readFileSync(join(WORKSPACE_ROOT, relativePath), 'utf8');
}

export function uniqueSorted(values: string[]): string[] {
  return Array.from(new Set(values)).sort((left, right) => left.localeCompare(right));
}

export function diffCommands(left: string[], right: string[]): CommandDiff {
  const rightSet = new Set(right);
  const leftSet = new Set(left);

  return {
    leftOnly: left.filter((command) => !rightSet.has(command)),
    rightOnly: right.filter((command) => !leftSet.has(command)),
  };
}

export function findDuplicates(values: string[]): string[] {
  const seen = new Set<string>();
  const duplicates = new Set<string>();

  for (const value of values) {
    if (seen.has(value)) {
      duplicates.add(value);
      continue;
    }

    seen.add(value);
  }

  return Array.from(duplicates).sort();
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

export function parseCollectCommands(source: string, occurrence: number): string[] {
  const openingBracket = collectMacroStart(source, occurrence);
  const body = extractBracketBody(source, openingBracket);
  return parseRustCommandNames(body);
}

export function countOccurrences(source: string, pattern: string): number {
  let count = 0;
  let searchFrom = 0;

  while (searchFrom < source.length) {
    const index = source.indexOf(pattern, searchFrom);
    if (index === -1) {
      return count;
    }

    count += 1;
    searchFrom = index + pattern.length;
  }

  return count;
}

export function parsePermissionCommands(source: string): string[] {
  const declaration = 'commands.allow = [';
  const start = source.indexOf(declaration);
  if (start === -1) {
    throw new Error('Unable to find permissions command allow list');
  }

  const openingBracket = start + declaration.length - 1;
  const body = extractBracketBody(source, openingBracket);
  return uniqueSorted(Array.from(body.matchAll(/"([a-zA-Z0-9_]+)"/g), (match) => match[1]));
}

export function parseAllPermissionCommands(): string[] {
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

export function parseBindingInvokeCommands(source: string): string[] {
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

function extractCommandObjectBody(source: string): string {
  const declaration = 'export const commands = {';
  const start = source.indexOf(declaration);
  if (start === -1) {
    throw new Error('Unable to find frontend command registry');
  }

  let depth = 0;
  const openingBrace = start + declaration.length - 1;
  for (let index = openingBrace; index < source.length; index += 1) {
    const char = source[index];
    if (char === '{') {
      depth += 1;
      continue;
    }

    if (char !== '}') {
      continue;
    }

    depth -= 1;
    if (depth === 0) {
      return source.slice(openingBrace + 1, index);
    }
  }

  throw new Error('Unable to find frontend command registry closing brace');
}

export function parseCommandWrapperNames(source: string): string[] {
  const body = extractCommandObjectBody(source);
  return uniqueSorted(
    Array.from(body.matchAll(/^\s{2}([a-zA-Z0-9_]+):\s*\(/gm), (match) => match[1]),
  );
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function isProductionSourceFile(relativePath: string): boolean {
  if (relativePath === BINDINGS_PATH) {
    return false;
  }

  if (relativePath.endsWith('.test.ts') || relativePath.endsWith('.test.tsx')) {
    return false;
  }

  if (relativePath.endsWith('.audit.test.ts') || relativePath.endsWith('.audit.test.tsx')) {
    return false;
  }

  if (relativePath.endsWith('setupTests.ts')) {
    return false;
  }

  return relativePath.endsWith('.ts') || relativePath.endsWith('.tsx');
}

function readSourceFilesFromDirectory(relativeDirectory: string): SourceFile[] {
  const absoluteDirectory = join(WORKSPACE_ROOT, relativeDirectory);
  const entries = readdirSync(absoluteDirectory, { withFileTypes: true });
  const files: SourceFile[] = [];

  for (const entry of entries) {
    const relativePath = `${relativeDirectory}/${entry.name}`;
    if (entry.isDirectory()) {
      if (EXCLUDED_SOURCE_DIRECTORIES.has(entry.name)) {
        continue;
      }

      files.push(...readSourceFilesFromDirectory(relativePath));
      continue;
    }

    if (!entry.isFile() || !isProductionSourceFile(relativePath)) {
      continue;
    }

    files.push({
      relativePath,
      source: readWorkspaceFile(relativePath),
    });
  }

  return files;
}

export function readProductionSourceFiles(): SourceFile[] {
  return SOURCE_DIRECTORIES.flatMap((directory) => readSourceFilesFromDirectory(directory));
}

export function commandWrapperHasCaller(wrapperName: string, sourceFiles: SourceFile[]): boolean {
  const pattern = new RegExp(`commands\\s*\\.\\s*${escapeRegExp(wrapperName)}\\b`);
  return sourceFiles.some((file) => pattern.test(file.source));
}
