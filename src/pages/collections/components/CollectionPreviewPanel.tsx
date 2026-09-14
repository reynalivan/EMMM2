/**
 * CollectionPreviewPanel — Right sidebar showing collection members.
 *
 * Extracted from CollectionsPage. Uses useCollectionPreview hook directly.
 * Replaces CollectionWorkspace for the preview use case.
 */

import { Layers, Package } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useCollectionPreview } from '../hooks/useCollections';
import { CollectionTreeView } from './CollectionTreeView';
import { getCollectionDisplayName, useRuntimeLabels } from '../../../shared/lib/runtimeLabels';
import type { CollectionRuntimeSnapshot } from '@/entities/collection';
import type { CollectionWorkspaceSource } from '../types';
import WorkspacePanelSkeleton from '@/shared/ui/components/ui/WorkspacePanelSkeleton';

interface CollectionPreviewPanelProps {
  source: CollectionWorkspaceSource | null;
  gameId: string | null;
  runtimeSnapshot: CollectionRuntimeSnapshot | undefined;
}

export function CollectionPreviewPanel({
  source,
  gameId,
  runtimeSnapshot,
}: CollectionPreviewPanelProps) {
  const { t } = useTranslation(['collections', 'common', 'layout']);
  const previewQuery = useCollectionPreview(
    source?.kind === 'stored_collection' ? source.collectionId : null,
    gameId,
  );
  const runtimeLabels = useRuntimeLabels();
  const [treeScrollElement, setTreeScrollElement] = useState<HTMLDivElement | null>(null);

  // No collection selected
  if (!source) {
    return (
      <div className="flex flex-col items-center justify-center p-8 text-center h-full">
        <div className="w-20 h-20 rounded-full bg-base-300 flex items-center justify-center mb-6 text-base-content/20 shadow-inner">
          <Layers size={40} className="opacity-50" />
        </div>
        <h3 className="text-xl font-bold opacity-80 mb-2">{t('collections:preview.title')}</h3>
        <p className="text-base-content/50 max-w-sm leading-relaxed">
          {t('collections:preview.no_selection')}
        </p>
      </div>
    );
  }

  // Loading
  if (source.kind === 'stored_collection' && previewQuery.isLoading) {
    return (
      <div className="flex h-full min-h-125 flex-col" aria-busy="true" role="status">
        <WorkspacePanelSkeleton variant="preview" />
        <p className="px-6 pb-6 text-center text-sm text-base-content/50">
          {t('common:status.loading')}
        </p>
      </div>
    );
  }

  if (source.kind === 'current_runtime') {
    if (!runtimeSnapshot) {
      return (
        <div className="flex flex-col items-center justify-center p-8 text-center h-full text-base-content/40">
          <Package size={48} className="mb-4 opacity-20" />
          <p>{t('common:status.not_found')}</p>
        </div>
      );
    }

    const displayName = getCollectionDisplayName({
      name: runtimeSnapshot.is_dirty ? null : runtimeSnapshot.active_collection_name,
      isUnsaved: runtimeSnapshot.is_dirty,
      labels: runtimeLabels,
    });

    return (
      <div className="relative flex h-full min-h-0 w-full flex-col">
        <div className="z-10 flex h-14 shrink-0 items-center justify-between border-b border-base-content/5 bg-base-300 px-4">
          <div className="flex items-center gap-3 min-w-0 flex-1">
            <div className="flex flex-col min-w-0">
              <h2 className="font-bold text-sm leading-tight flex items-center gap-2 truncate">
                <span className="truncate">{displayName}</span>
                <span className="badge badge-sm badge-success opacity-90 text-[10px] py-0 h-4 uppercase font-bold tracking-wider shrink-0">
                  {t('collections:list.item.active')}
                </span>
              </h2>
              <span className="text-[10px] text-base-content/50 truncate">
                {t('collections:preview.stats.mods', {
                  count: runtimeSnapshot.projected_state.summary.active_root_count,
                })}
              </span>
            </div>
          </div>
        </div>

        <div
          ref={setTreeScrollElement}
          className="flex-1 overflow-y-auto custom-scrollbar p-4 bg-base-100/50"
        >
          <div className="max-w-3xl mx-auto">
            <CollectionTreeView
              nodes={runtimeSnapshot.current_tree_nodes}
              gameId={gameId}
              colorClass="text-primary"
              emptyMessage={t('collections:preview.empty')}
              scrollElement={treeScrollElement}
              treeIdentity={`runtime:${gameId ?? ''}`}
            />
          </div>
        </div>
      </div>
    );
  }

  const preview = previewQuery.data;
  if (!preview) {
    return (
      <div className="flex flex-col items-center justify-center p-8 text-center h-full text-base-content/40">
        <Package size={48} className="mb-4 opacity-20" />
        <p>{t('common:status.not_found')}</p>
      </div>
    );
  }

  return (
    <div className="relative flex h-full min-h-0 w-full flex-col">
      {/* Header */}
      <div className="z-10 flex h-14 shrink-0 items-center justify-between border-b border-base-content/5 bg-base-300 px-4">
        <div className="flex items-center gap-3 min-w-0 flex-1">
          <div className="flex flex-col min-w-0">
            <h2 className="font-bold text-sm leading-tight flex items-center gap-2 truncate">
              <span className="truncate">{preview.collection.name}</span>
              {preview.collection.is_active && (
                <span className="badge badge-sm badge-success opacity-90 text-[10px] py-0 h-4 uppercase font-bold tracking-wider shrink-0">
                  {t('collections:list.item.active')}
                </span>
              )}
            </h2>
            <span className="text-[10px] text-base-content/50 truncate">
              {t('collections:preview.stats.mods', {
                count: preview.projected_state.summary.active_root_count,
              })}
            </span>
          </div>
        </div>
      </div>

      {/* Tree view */}
      <div
        ref={setTreeScrollElement}
        className="flex-1 overflow-y-auto custom-scrollbar p-4 bg-base-100/50"
      >
        <div className="max-w-3xl mx-auto">
          <CollectionTreeView
            nodes={preview.tree_nodes}
            gameId={gameId}
            colorClass="text-primary"
            emptyMessage={t('collections:preview.empty')}
            scrollElement={treeScrollElement}
            treeIdentity={`collection:${source.collectionId}`}
          />
        </div>
      </div>
    </div>
  );
}
