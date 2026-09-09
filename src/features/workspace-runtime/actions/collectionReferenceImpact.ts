import type { QueryClient } from '@tanstack/react-query';
import type { CollectionReferenceImpact } from '@/entities/collection';
import { toast } from '@/shared/ui/toast';
import { publishRuntimeDescriptor } from '@/shared/lib/queryRefresh';
import { buildRuntimeMutationDescriptor } from '../optimistic/descriptorBuilders';
import type { RuntimeRefreshEvent } from '@/shared/lib/runtimeEffects';

export function hasCollectionReferenceImpact(
  impact: CollectionReferenceImpact | null | undefined,
): boolean {
  if (!impact) {
    return false;
  }

  return impact.affected_collection_count > 0;
}

export function formatCollectionReferenceImpact(impact: CollectionReferenceImpact): string | null {
  if (!hasCollectionReferenceImpact(impact)) {
    return null;
  }

  const names = impact.affected_collection_names.slice(0, 3).join(', ');
  const extraCount = Math.max(0, impact.affected_collection_names.length - 3);
  const suffix = extraCount > 0 ? `, +${extraCount} more` : '';
  const collectionLabel = impact.affected_collection_count === 1 ? 'collection' : 'collections';

  if (impact.rewritten_paths.length > 0) {
    return `Updated references in ${impact.affected_collection_count} ${collectionLabel}: ${names}${suffix}`;
  }

  if (impact.missing_paths.length > 0) {
    return `${impact.affected_collection_count} ${collectionLabel} now reference missing files: ${names}${suffix}`;
  }

  return null;
}

export function notifyCollectionReferenceImpact(impact: CollectionReferenceImpact): void {
  const message = formatCollectionReferenceImpact(impact);
  if (message) {
    toast.info(message, 5000);
  }
}

/** Extra refresh event needed when a mutation rewrites collection references. */
export function collectionReferenceImpactRefreshEvents(
  impact: CollectionReferenceImpact | null | undefined,
): RuntimeRefreshEvent[] {
  return hasCollectionReferenceImpact(impact) ? ['collectionsChanged'] : [];
}

/**
 * Republish the collections catalog and tell the user which collections a
 * mutation touched. Every mutation returning a `collection_impact` ends this way.
 */
export async function publishCollectionReferenceImpact(
  queryClient: QueryClient,
  impact: CollectionReferenceImpact | null | undefined,
): Promise<void> {
  if (!impact || !hasCollectionReferenceImpact(impact)) {
    return;
  }

  await publishRuntimeDescriptor(
    queryClient,
    buildRuntimeMutationDescriptor('collectionsCatalog'),
    'active',
  );
  notifyCollectionReferenceImpact(impact);
}
