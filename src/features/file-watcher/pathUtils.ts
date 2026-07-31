import { pathsEqual, pathStartsWith } from '../../lib/pathKey';
import { normalizeWorkspacePath } from '../workspace-runtime/pathRewrite';

export function joinModPath(modsPath: string, relativePath: string): string {
  const base = normalizeWorkspacePath(modsPath);
  const relative = normalizeWorkspacePath(relativePath).replace(/^\/+/, '');
  return relative ? `${base}/${relative}` : base;
}

export function rewritePath(
  candidatePath: string,
  fromPath: string,
  toPath: string,
): string | null {
  const candidate = normalizeWorkspacePath(candidatePath);
  const from = normalizeWorkspacePath(fromPath);
  const to = normalizeWorkspacePath(toPath);

  if (pathsEqual(candidate, from)) {
    return to;
  }

  if (!pathStartsWith(from, candidate)) {
    return null;
  }

  return `${to}${candidate.slice(from.length)}`;
}
