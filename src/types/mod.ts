/**
 * Frontend-only vocabulary for the mod explorer.
 *
 * Wire types are NOT re-exported here — import them from their real source:
 * `types/object` (folders, info.json, conflicts) or `types/scanner` (bulk
 * results, trash, dup scan), both of which sit directly on `bindings.gen`.
 */

import type { TrashMetadata } from './scanner';

/** Metadata for a trashed mod folder. */
export type TrashEntry = TrashMetadata;

/** Sort field for mod folder listing. */
export type SortField = 'name' | 'modified_at' | 'size_bytes';

/** Sort direction. */
export type SortOrder = 'asc' | 'desc';

/** Explorer view mode. */
export type ViewMode = 'grid' | 'list';
