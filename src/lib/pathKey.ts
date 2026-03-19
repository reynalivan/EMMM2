function asciiCaseFold(value: string): string {
  let result = '';
  for (const char of value) {
    if (char >= 'A' && char <= 'Z') {
      result += char.toLowerCase();
      continue;
    }
    result += char;
  }
  return result;
}

export function canonicalPathKey(value: string | null | undefined): string | null {
  if (!value) {
    return null;
  }

  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }

  const normalized = trimmed.replace(/\\/g, '/');
  const segments = normalized.split(/\/+/).filter(Boolean);
  if (segments.length === 0) {
    return null;
  }

  return segments.map((segment) => asciiCaseFold(segment)).join('/');
}

export function pathsEqual(
  left: string | null | undefined,
  right: string | null | undefined,
): boolean {
  const leftKey = canonicalPathKey(left);
  const rightKey = canonicalPathKey(right);
  if (!leftKey || !rightKey) {
    return false;
  }

  return leftKey === rightKey;
}

export function pathStartsWith(
  parent: string | null | undefined,
  child: string | null | undefined,
): boolean {
  const parentKey = canonicalPathKey(parent);
  const childKey = canonicalPathKey(child);
  if (!parentKey || !childKey) {
    return false;
  }

  if (parentKey === childKey) {
    return true;
  }

  return childKey.startsWith(`${parentKey}/`);
}

export function relativePathFromRoot(
  root: string | null | undefined,
  target: string | null | undefined,
): string | null {
  const rootKey = canonicalPathKey(root);
  const targetKey = canonicalPathKey(target);
  if (!rootKey || !targetKey) {
    return null;
  }

  if (rootKey === targetKey) {
    return '';
  }

  if (!targetKey.startsWith(`${rootKey}/`)) {
    return null;
  }

  return targetKey.slice(rootKey.length + 1);
}
