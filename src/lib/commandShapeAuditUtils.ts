/**
 * Parsers that compare the hand-written `commands` wrappers in `bindings.ts`
 * against the tauri-specta output in `bindings.gen.ts`.
 *
 * Name parity is covered by `commandRegistry.audit.test.ts`. This module covers
 * *shape* parity: a wrapper that marks a required Rust argument optional (or
 * invents an argument name Rust never reads) produces a payload that fails serde
 * at runtime, which surfaces as a rejected promise long after type-checking passed.
 */

export type GeneratedArgument = {
  name: string;
  nullable: boolean;
};

export type WrapperParameter = {
  name: string;
  optional: boolean;
};

const GENERATED_COMMAND = /async (\w+)\(([^)]*)\)[\s\S]*?TAURI_INVOKE\("([a-z0-9_]+)"/g;
const WRAPPER_ENTRY = /^ {2}([A-Za-z0-9_]+): \(/gm;
const WRAPPER_COMMAND = /invoke<[\s\S]*?>\('([a-z0-9_]+)'/;

const OPENING = new Set(['{', '[', '(', '<']);
const CLOSING = new Set(['}', ']', ')', '>']);

/** Split on separators that sit outside any bracket pair. */
function splitTopLevel(source: string, separator: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let current = '';

  for (const char of source) {
    if (OPENING.has(char)) depth += 1;
    else if (CLOSING.has(char)) depth -= 1;

    if (char === separator && depth === 0) {
      parts.push(current);
      current = '';
      continue;
    }

    current += char;
  }

  parts.push(current);
  return parts.map((part) => part.trim()).filter((part) => part.length > 0);
}

/** Return the substring inside the brace pair opening at or after `from`. */
function extractBraceBody(source: string, from: number): string | null {
  const start = source.indexOf('{', from);
  if (start === -1) return null;

  let depth = 0;
  for (let index = start; index < source.length; index += 1) {
    if (source[index] === '{') depth += 1;
    else if (source[index] === '}') {
      depth -= 1;
      if (depth === 0) return source.slice(start + 1, index);
    }
  }

  return null;
}

/** Rust command name -> arguments tauri-specta generated for it. */
export function parseGeneratedCommandArgs(source: string): Map<string, GeneratedArgument[]> {
  const commands = new Map<string, GeneratedArgument[]>();

  for (const match of source.matchAll(GENERATED_COMMAND)) {
    const [, , rawArgs, commandName] = match;
    const args = splitTopLevel(rawArgs, ',').map((arg) => {
      const separator = arg.indexOf(':');
      const name = arg.slice(0, separator).trim();
      const type = arg.slice(separator + 1).trim();
      return { name, nullable: /\bnull\b/.test(type) };
    });

    commands.set(commandName, args);
  }

  return commands;
}

/** Rust command name -> parameters the hand-written wrapper accepts. */
export function parseWrapperCommandParams(source: string): Map<string, WrapperParameter[]> {
  const wrappers = new Map<string, WrapperParameter[]>();
  const starts = Array.from(source.matchAll(WRAPPER_ENTRY), (match) => match.index ?? 0);

  for (const [position, start] of starts.entries()) {
    const entry = source.slice(start, starts[position + 1] ?? source.length);
    const commandName = entry.match(WRAPPER_COMMAND)?.[1];
    if (!commandName) continue;

    const paramsAt = entry.indexOf('(params:');
    if (paramsAt === -1) {
      wrappers.set(commandName, []);
      continue;
    }

    const body = extractBraceBody(entry, paramsAt);
    const params = body
      ? splitTopLevel(body, ';').map((field) => {
          const name = field.slice(0, field.indexOf(':')).trim();
          return { name: name.replace(/\?$/, ''), optional: name.endsWith('?') };
        })
      : [];

    wrappers.set(commandName, params);
  }

  return wrappers;
}
