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

/**
 * Trailing segment of a path, separator-agnostic. Returns the input unchanged
 * when it holds no segments, so callers can use it for display names directly.
 */
export function pathBasename(value: string): string {
  const segments = value.split(/[\\/]/).filter(Boolean);
  return segments[segments.length - 1] ?? value;
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
