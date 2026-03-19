import { canonicalPathKey, pathStartsWith, pathsEqual } from '../../lib/pathKey';
import type { ModFolder } from '../../types/mod';

export function resolvePreviewTargetPath(
  externalSelectedPath: string | null,
  selfModPath: string | null,
  children: ModFolder[],
): string | null {
  if (!externalSelectedPath) {
    return selfModPath;
  }

  if (!selfModPath) {
    return externalSelectedPath;
  }

  const normalizedExternal = canonicalPathKey(externalSelectedPath);
  const normalizedSelf = canonicalPathKey(selfModPath);

  if (!normalizedExternal || !normalizedSelf) {
    return externalSelectedPath;
  }

  if (pathsEqual(externalSelectedPath, selfModPath)) {
    return selfModPath;
  }

  if (!pathStartsWith(selfModPath, externalSelectedPath)) {
    return externalSelectedPath;
  }

  const selectedChild = children.find(
    (folder) => pathsEqual(folder.path, externalSelectedPath),
  );

  if (!selectedChild) {
    return selfModPath;
  }

  if (selectedChild.node_type === 'ContainerFolder') {
    return externalSelectedPath;
  }

  return selfModPath;
}
