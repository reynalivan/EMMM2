/** Pure helpers for bulk ObjectList operations and their toast copy. */

/** Resolves selected ids to display names, falling back to the raw id. */
export function resolveObjectNames(
  ids: Iterable<string>,
  objects: Array<{ id: string; name: string }>,
): string[] {
  return Array.from(ids, (id) => objects.find((object) => object.id === id)?.name ?? id);
}

/** Object tags are stored as a JSON array string; bad data reads as no tags. */
export function parseTagList(raw: string | null | undefined): string[] {
  if (!raw) {
    return [];
  }

  try {
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((tag): tag is string => typeof tag === 'string') : [];
  } catch {
    return [];
  }
}
