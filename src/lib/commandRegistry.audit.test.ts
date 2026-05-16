import { describe, expect, it } from 'vitest';
import {
  commandWrapperHasCaller,
  countOccurrences,
  diffCommands,
  findDuplicates,
  parseAllPermissionCommands,
  parseBindingInvokeCommands,
  parseCollectCommands,
  parseCommandWrapperNames,
  parsePermissionCommands,
  readProductionSourceFiles,
  readWorkspaceFile,
  uniqueSorted,
} from './commandRegistryAuditUtils';

const LIB_RS_PATH = 'src-tauri/src/lib.rs';
const PERMISSIONS_PATH = 'src-tauri/permissions/app-commands.toml';
const BINDINGS_PATH = 'src/lib/bindings.ts';
const COLLECT_COMMANDS_MACRO = 'collect_commands![';
const BACKEND_ONLY_COMMANDS: string[] = ['move_mod_to_object'];
const COMMAND_WRAPPER_USAGE_POLICY: string[] = [];

function expectNoDuplicates(label: string, values: string[]): void {
  expect(findDuplicates(values), `${label} contains duplicate commands`).toEqual([]);
}

function assertSameCommands(label: string, left: string[], right: string[]): void {
  const diff = diffCommands(left, right);

  expect(diff.leftOnly, `${label}: left-only commands`).toEqual([]);
  expect(diff.rightOnly, `${label}: right-only commands`).toEqual([]);
}

function removedCommandNames(): string[] {
  return [
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
    ['list', 'mod', 'folders'].join('_'),
    ['pre', 'delete', 'check'].join('_'),
    ['toggle', 'favorite'].join('_'),
    ['get', 'thumbnail'].join('_'),
    ['check', 'shader', 'conflicts'].join('_'),
    ['clear', 'pending', 'tasks'].join('_'),
    ['browser', 'close', 'tab'].join('_'),
    ['remove', 'game'].join('_'),
  ];
}

describe('command registry audit', () => {
  const libSource = readWorkspaceFile(LIB_RS_PATH);
  const permissionSource = readWorkspaceFile(PERMISSIONS_PATH);
  const bindingSource = readWorkspaceFile(BINDINGS_PATH);
  const productionCommands = parseCollectCommands(libSource, 0);
  const spectaCommands = productionCommands;
  const permissionCommands = parsePermissionCommands(permissionSource);
  const allPermissionCommands = parseAllPermissionCommands();
  const bindingCommands = parseBindingInvokeCommands(bindingSource);
  const commandWrapperNames = parseCommandWrapperNames(bindingSource);
  const productionSourceFiles = readProductionSourceFiles();

  it('uses one shared Rust command registry for production and Specta export', () => {
    expect(countOccurrences(libSource, COLLECT_COMMANDS_MACRO)).toBe(1);
    expect(countOccurrences(libSource, '.commands(emmm_collect_commands!())')).toBe(2);
  });

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

  it('requires frontend command wrappers to have non-test callers or be explicitly classified', () => {
    const policySet = new Set(COMMAND_WRAPPER_USAGE_POLICY);
    const unusedWrappers = commandWrapperNames.filter(
      (wrapperName) =>
        !policySet.has(wrapperName) && !commandWrapperHasCaller(wrapperName, productionSourceFiles),
    );

    expectNoDuplicates('frontend command wrapper policy', COMMAND_WRAPPER_USAGE_POLICY);
    expect(unusedWrappers).toEqual([]);
  });

  it('does not expose removed frontend command aliases', () => {
    const staleAliases = ['get' + 'LogLines', 'get' + 'Game', 'get' + 'WatcherState'];
    const offenders = staleAliases.filter((alias) => bindingSource.includes(`${alias}:`));

    expect(offenders).toEqual([]);
  });

  it('does not keep removed legacy command names in active command registries', () => {
    const activeCommandNames = uniqueSorted([
      ...productionCommands,
      ...spectaCommands,
      ...permissionCommands,
      ...bindingCommands,
    ]);
    const offenders = removedCommandNames().filter((command) =>
      activeCommandNames.includes(command),
    );

    expect(offenders).toEqual([]);
  });
});
