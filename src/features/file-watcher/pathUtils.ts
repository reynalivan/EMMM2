export function normalizePath(path: string): string {
  return path.replace(/\\/g, '/').replace(/\/+$/g, '');
}

export function joinModPath(modsPath: string, relativePath: string): string {
  const base = normalizePath(modsPath);
  const relative = normalizePath(relativePath).replace(/^\/+/g, '');
  return relative ? `${base}/${relative}` : base;
}

export function isSameOrDescendantPath(candidatePath: string, rootPath: string): boolean {
  const normalizedCandidate = normalizePath(candidatePath);
  const normalizedRoot = normalizePath(rootPath);

  if (!normalizedRoot) {
    return false;
  }

  return (
    normalizedCandidate === normalizedRoot || normalizedCandidate.startsWith(`${normalizedRoot}/`)
  );
}

export function rewritePath(candidatePath: string, fromPath: string, toPath: string): string | null {
  const normalizedCandidate = normalizePath(candidatePath);
  const normalizedFrom = normalizePath(fromPath);
  const normalizedTo = normalizePath(toPath);

  if (normalizedCandidate === normalizedFrom) {
    return normalizedTo;
  }

  if (!normalizedCandidate.startsWith(`${normalizedFrom}/`)) {
    return null;
  }

  return `${normalizedTo}${normalizedCandidate.slice(normalizedFrom.length)}`;
}

export function isModFolder(path: string, modsPath: string): boolean {
  const normalizedPath = normalizePath(path).toLowerCase();
  const normalizedModsPath = normalizePath(modsPath).toLowerCase();
  const prefix = `${normalizedModsPath}/`;

  if (!normalizedPath.startsWith(prefix)) {
    return false;
  }

  const relative = normalizedPath.slice(prefix.length);
  const segments = relative.split('/').filter(Boolean);
  if (segments.length !== 2) {
    return false;
  }

  return !segments[1].includes('.');
}
