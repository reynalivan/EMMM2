export interface WorkspacePathRewriteInput {
  oldPath: string;
  newPath: string;
}

export function normalizeWorkspacePath(path: string): string {
  return path.replace(/\\/g, '/');
}

export function rewriteWorkspacePathValue(
  value: string | null | undefined,
  rewrites: WorkspacePathRewriteInput[],
): string | null | undefined {
  if (!value) {
    return value;
  }

  let nextValue = normalizeWorkspacePath(value);
  for (const rewrite of rewrites) {
    const oldPath = normalizeWorkspacePath(rewrite.oldPath);
    const newPath = normalizeWorkspacePath(rewrite.newPath);
    const oldName = oldPath.split('/').filter(Boolean).pop() ?? oldPath;
    const newName = newPath.split('/').filter(Boolean).pop() ?? newPath;

    if (nextValue === oldPath) {
      nextValue = newPath;
      continue;
    }

    if (nextValue.startsWith(`${oldPath}/`)) {
      nextValue = `${newPath}${nextValue.slice(oldPath.length)}`;
      continue;
    }

    if (nextValue === oldName) {
      nextValue = newName;
      continue;
    }

    if (nextValue.startsWith(`${oldName}/`)) {
      nextValue = `${newName}${nextValue.slice(oldName.length)}`;
      continue;
    }

    const segments = nextValue.split('/');
    if (segments.includes(oldName)) {
      nextValue = segments.map((segment) => (segment === oldName ? newName : segment)).join('/');
    }
  }

  return nextValue;
}
