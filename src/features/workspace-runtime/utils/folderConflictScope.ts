import { normalizeWorkspacePath } from './pathRewrite';

function comparablePath(path: string): string {
  return normalizeWorkspacePath(path).toLocaleLowerCase('en-US');
}

export function isFolderConflictProtected(path: string, scopes: readonly string[]): boolean {
  const candidate = comparablePath(path);
  return scopes.some((scope) => {
    const normalizedScope = comparablePath(scope);
    return candidate === normalizedScope || candidate.startsWith(`${normalizedScope}/`);
  });
}
