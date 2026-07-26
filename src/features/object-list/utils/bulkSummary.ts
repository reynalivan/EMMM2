/** Pure helpers for bulk ObjectList operations and their toast copy. */

const MAX_LISTED_NAMES = 4;

/** Resolves selected ids to display names, falling back to the raw id. */
export function resolveObjectNames(
  ids: Iterable<string>,
  objects: Array<{ id: string; name: string }>,
): string[] {
  return Array.from(ids, (id) => objects.find((object) => object.id === id)?.name ?? id);
}

/**
 * "a, b, c, d <more(n)>" — long selections are truncated so a bulk toast stays
 * readable. `more` renders the tail because callers word it differently.
 */
export function truncateNameList(names: string[], more: (extra: number) => string): string {
  if (names.length <= MAX_LISTED_NAMES) {
    return names.join(', ');
  }

  return `${names.slice(0, MAX_LISTED_NAMES).join(', ')} ${more(names.length - MAX_LISTED_NAMES)}`;
}

/** Object tags are stored as a JSON array string; bad data reads as no tags. */
export function parseTagList(raw: string | null | undefined): string[] {
  try {
    return JSON.parse(raw || '[]');
  } catch {
    return [];
  }
}
