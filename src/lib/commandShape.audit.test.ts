import { describe, expect, it } from 'vitest';
import { readWorkspaceFile } from './commandRegistryAuditUtils';
import { parseGeneratedCommandArgs, parseWrapperCommandParams } from './commandShapeAuditUtils';

const BINDINGS_PATH = 'src/lib/bindings.ts';
const GENERATED_PATH = 'src/lib/bindings.gen.ts';

/**
 * `bindings.gen.ts` is the tauri-specta export of the real Rust signatures
 * (regenerate with `cargo test specta_tests::export_bindings`). A wrapper that
 * drifts from it compiles fine and then fails serde at runtime.
 */
describe('command wrapper payload shapes', () => {
  const generated = parseGeneratedCommandArgs(readWorkspaceFile(GENERATED_PATH));
  const wrappers = parseWrapperCommandParams(readWorkspaceFile(BINDINGS_PATH));

  it('parses both registries', () => {
    expect(generated.size).toBeGreaterThan(100);
    expect(wrappers.size).toBeGreaterThan(100);
  });

  it('never marks a required Rust argument optional', () => {
    const offenders: string[] = [];

    for (const [command, params] of wrappers) {
      const args = generated.get(command);
      if (!args) continue;

      for (const arg of args.filter((candidate) => !candidate.nullable)) {
        const param = params.find((candidate) => candidate.name === arg.name);
        if (!param) {
          offenders.push(`${command}: missing required '${arg.name}'`);
          continue;
        }
        if (param.optional) {
          offenders.push(`${command}: '${arg.name}' is required in Rust but optional here`);
        }
      }
    }

    expect(offenders).toEqual([]);
  });

  it('never sends a field the Rust command does not read', () => {
    const offenders: string[] = [];

    for (const [command, params] of wrappers) {
      const args = generated.get(command);
      if (!args) continue;

      const known = new Set(args.map((arg) => arg.name));
      for (const param of params) {
        if (!known.has(param.name)) {
          offenders.push(`${command}: '${param.name}' is not an argument of this command`);
        }
      }
    }

    expect(offenders).toEqual([]);
  });
});
