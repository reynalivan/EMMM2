import type { FilterDef } from '../../types/object';

export type ObjectMetaFilters = Record<string, string[]>;

function areStringArraysEqual(left: string[], right: string[]): boolean {
  if (left.length !== right.length) {
    return false;
  }

  return left.every((value, index) => value === right[index]);
}

export function areObjectMetaFiltersEqual(
  left: ObjectMetaFilters | null | undefined,
  right: ObjectMetaFilters | null | undefined,
): boolean {
  const normalizedLeft = left ?? {};
  const normalizedRight = right ?? {};
  const leftKeys = Object.keys(normalizedLeft).sort();
  const rightKeys = Object.keys(normalizedRight).sort();

  if (!areStringArraysEqual(leftKeys, rightKeys)) {
    return false;
  }

  return leftKeys.every((key) => {
    const leftValues = normalizedLeft[key] ?? [];
    const rightValues = normalizedRight[key] ?? [];
    return areStringArraysEqual(leftValues, rightValues);
  });
}

export function sanitizeObjectMetaFilters(
  filters: ObjectMetaFilters | null | undefined,
  categoryFilters: FilterDef[],
): ObjectMetaFilters {
  const normalizedFilters = filters ?? {};
  if (categoryFilters.length === 0) {
    return { ...normalizedFilters };
  }

  const validKeys = new Set(categoryFilters.map((filter) => filter.key));
  const nextFilters: ObjectMetaFilters = {};

  for (const [key, values] of Object.entries(normalizedFilters)) {
    if (!validKeys.has(key) || values.length === 0) {
      continue;
    }

    nextFilters[key] = values;
  }

  return nextFilters;
}
