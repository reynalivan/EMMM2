/**
 * CollectionPreviewPanel — Right sidebar showing collection members.
 *
 * Extracted from CollectionsPage. Uses useCollectionPreview hook directly.
 * Replaces CollectionWorkspace for the preview use case.
 */

import { useMemo } from 'react';
import { Layers, Loader2, Package } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useCollectionPreview } from '../hooks';
import { CollectionTreeView } from './CollectionTreeView';
import type { CollectionMember } from '../../../types/collection';

interface CollectionPreviewPanelProps {
  collectionId: string | null;
  gameId: string | null;
}

export function CollectionPreviewPanel({ collectionId, gameId }: CollectionPreviewPanelProps) {
  const { t } = useTranslation(['collections', 'common']);
  const previewQuery = useCollectionPreview(collectionId, gameId);
  const treeMembers = useMemo<CollectionMember[]>(() => {
    if (!previewQuery.data?.members) return [];
    // Include objects, roots, and enabled mods — tree view handles hierarchy
    return previewQuery.data.members.filter(
      (m) => m.kind === 'object' || m.kind === 'root' || ((m.kind === 'mod' || m.kind === 'nested') && m.is_enabled),
    );
  }, [previewQuery.data]);

  // No collection selected
  if (!collectionId) {
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
  if (previewQuery.isLoading) {
    return (
      <div className="flex flex-col h-full items-center justify-center min-h-125">
        <Loader2 size={32} className="animate-spin text-primary opacity-50 mb-4" />
        <p className="text-base-content/50">{t('common:status.loading')}</p>
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
                {preview.collection.is_unsaved
                  ? t('context.unsaved', 'Unsaved Preset')
                  : preview.collection.name}
              </span>
              {preview.collection.is_active && (
                <span className="badge badge-sm badge-success opacity-90 text-[10px] py-0 h-4 uppercase font-bold tracking-wider shrink-0">
                  {t('collections:list.item.active')}
                </span>
              )}
            </h2>
            <span className="text-[10px] text-base-content/50 truncate">
              {t('collections:preview.stats.mods', {
                count: treeMembers.filter((m) => m.kind === 'mod' || m.kind === 'nested').length,
              })}
            </span>
          </div>
        </div>
      </div>

      {/* Members list — uses shared ModGroupList for consistent display */}
      <div className="flex-1 overflow-y-auto custom-scrollbar p-4 bg-base-100/50">
        <div className="max-w-3xl mx-auto">
          <CollectionTreeView
            members={treeMembers}
            colorClass="text-primary"
            emptyMessage={t('collections:preview.empty')}
          />
        </div>
      </div>
    </div>
  );
}
