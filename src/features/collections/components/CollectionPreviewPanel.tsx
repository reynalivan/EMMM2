/**
 * CollectionPreviewPanel — Right sidebar showing collection members.
 *
 * Extracted from CollectionsPage. Uses useCollectionPreview hook directly.
 * Replaces CollectionWorkspace for the preview use case.
 */

import { Layers, Loader2, Package } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useCollectionPreview } from '../hooks/useCollections';
import { CollectionTreeView } from './CollectionTreeView';
import { getCollectionDisplayName, useUnsavedLabels } from '../../../lib/corridorLabels';
import type { CorridorSnapshot } from '../../../types/collection';
import type { CollectionWorkspaceSource } from '../types';

interface CollectionPreviewPanelProps {
  source: CollectionWorkspaceSource | null;
  gameId: string | null;
  corridorSnapshot: CorridorSnapshot | undefined;
}

export function CollectionPreviewPanel({
  source,
  gameId,
  corridorSnapshot,
}: CollectionPreviewPanelProps) {
  const { t } = useTranslation(['collections', 'common', 'layout']);
  const previewQuery = useCollectionPreview(
    source?.kind === 'stored_collection' ? source.collectionId : null,
    gameId,
  );
  const unsavedLabels = useUnsavedLabels();

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
      <div className="flex flex-col h-full items-center justify-center min-h-125">
        <Loader2 size={32} className="animate-spin text-primary opacity-50 mb-4" />
        <p className="text-base-content/50">{t('common:status.loading')}</p>
      </div>
    );
  }

  if (source.kind === 'current_runtime') {
    if (!corridorSnapshot) {
      return (
        <div className="flex flex-col items-center justify-center p-8 text-center h-full text-base-content/40">
          <Package size={48} className="mb-4 opacity-20" />
          <p>{t('common:status.not_found')}</p>
        </div>
      );
    }

    const displayName = getCollectionDisplayName({
      name: corridorSnapshot.is_dirty ? null : corridorSnapshot.active_collection_name,
      isUnsaved: corridorSnapshot.is_dirty || corridorSnapshot.active_collection_is_unsaved,
      isSafe: corridorSnapshot.is_safe,
      labels: unsavedLabels,
    });

    return (
      <div className="flex flex-col h-full w-full relative">
        <div className="h-14 bg-base-300/50 backdrop-blur-md border-b border-base-content/5 flex items-center justify-between px-4 shrink-0 z-10">
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
                    count: corridorSnapshot.projected_state.summary.active_root_count,
                  })}
                </span>
            </div>
          </div>
        </div>

        <div className="flex-1 overflow-y-auto custom-scrollbar p-4 bg-base-100/50">
          <div className="max-w-3xl mx-auto">
            <CollectionTreeView
              nodes={corridorSnapshot.current_tree_nodes}
              colorClass="text-primary"
              emptyMessage={t('collections:preview.empty')}
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
    <div className="flex flex-col h-full w-full relative">
      {/* Header */}
      <div className="h-14 bg-base-300/50 backdrop-blur-md border-b border-base-content/5 flex items-center justify-between px-4 shrink-0 z-10">
        <div className="flex items-center gap-3 min-w-0 flex-1">
          <div className="flex flex-col min-w-0">
            <h2 className="font-bold text-sm leading-tight flex items-center gap-2 truncate">
              <span className="truncate">
                {getCollectionDisplayName({
                  name: preview.collection.name,
                  isUnsaved: preview.collection.is_unsaved,
                  isSafe: preview.collection.is_safe,
                  labels: unsavedLabels,
                })}
              </span>
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
      <div className="flex-1 overflow-y-auto custom-scrollbar p-4 bg-base-100/50">
        <div className="max-w-3xl mx-auto">
          <CollectionTreeView
            nodes={preview.tree_nodes}
            colorClass="text-primary"
            emptyMessage={t('collections:preview.empty')}
          />
        </div>
      </div>
    </div>
  );
}
