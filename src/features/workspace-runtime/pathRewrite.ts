export interface WorkspacePathRewriteInput {
  oldPath: string;
  newPath: string;
}

// ponytail: single path normalizer for the whole app. Trailing slashes are
// stripped because every consumer prefix-matches against `${base}/`, and a
// retained trailing slash silently turns that into a never-matching `base//`.
export function normalizeWorkspacePath(path: string): string {
  return path.replace(/\\/g, '/').replace(/\/+$/, '');
}

/** ASCII-only case fold, mirroring the backend's path-key comparison rule. */
function foldCase(value: string): string {
  return value.replace(/[A-Z]/g, (char) => char.toLowerCase());
}

export function rewriteWorkspacePathValue(
  value: string | null | undefined,
  rewrites: WorkspacePathRewriteInput[],
): string | null | undefined {
  if (!value) {
    return value;
  }

  // Preserve the caller's separator style: selections are matched by exact
  // string against backend paths (backslashed on Windows), so a rewrite that
  // silently switched to forward slashes would break every Set lookup.
  const usedBackslash = value.includes('\\');

  // Matching is ASCII-case-insensitive (Windows paths; mirrors the backend
  // identity rule) while every emitted piece keeps its original casing.
  let nextValue = normalizeWorkspacePath(value);
  for (const rewrite of rewrites) {
    const oldPath = normalizeWorkspacePath(rewrite.oldPath);
    const newPath = normalizeWorkspacePath(rewrite.newPath);
    const foldedValue = foldCase(nextValue);
    const foldedOld = foldCase(oldPath);

    if (foldedValue === foldedOld) {
      nextValue = newPath;
      continue;
    }

    if (foldedValue.startsWith(`${foldedOld}/`)) {
      nextValue = `${newPath}${nextValue.slice(oldPath.length)}`;
      continue;
    }

    // Relative spelling: the stored value can be relative to the mods root
    // while rewrites carry absolute paths. Anchor on a whole-segment SUFFIX
    // of the old path — a bare basename match would also rewrite same-named
    // mods under other objects ("Nahida/SkinA" when "Raiden/SkinA" moved).
    const segments = nextValue.split('/');
    for (let depth = segments.length; depth >= 1; depth--) {
      const relPrefix = foldCase(segments.slice(0, depth).join('/'));
      if (!foldedOld.endsWith(`/${relPrefix}`)) {
        continue;
      }
      const newRel = newPath.split('/').slice(-depth).join('/');
      nextValue = [newRel, ...segments.slice(depth)].join('/');
      break;
    }
  }

  return usedBackslash ? nextValue.replace(/\//g, '\\') : nextValue;
}
