/**
 * CollectionList — Left panel showing all collections for the current corridor.
 *
 * Extracted from CollectionsPage. Replaces the inline table + workspaceRows chain.
 * Uses v2 types directly — no intermediary transformation.
 */

import { Layers, Trash2, Edit2, Check, X, PlayCircle, Loader2, Save } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { getCollectionDisplayName, useUnsavedLabels } from '../../../lib/corridorLabels';
import type { CollectionListRow, CollectionSaveRequest } from '../types';
import { DeleteCollectionModal } from './DeleteCollectionModal';

interface CollectionListProps {
  rows: CollectionListRow[];
  selectedId: string | null;
  isLoading: boolean;
  safeMode: boolean;
  onSelect: (id: string) => void;
  onApply: (id: string, name: string) => void;
  onDelete: (id: string) => void;
  onRename: (id: string, newName: string) => void;
  onSave?: (request: CollectionSaveRequest) => void;
  isApplying: boolean;
  isDeleting: boolean;
}

export function CollectionList({
  rows,
  selectedId,
  isLoading,
  safeMode,
  onSelect,
  onApply,
  onDelete,
  onRename,
  onSave,
  isApplying,
  isDeleting,
}: CollectionListProps) {
  const { t } = useTranslation('collections');
  const unsavedLabels = useUnsavedLabels();
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState('');
  const [collectionToDelete, setCollectionToDelete] = useState<{ id: string; name: string } | null>(
    null,
  );


  const startEdit = (collectionId: string, name: string) => {
    setEditingId(collectionId);
    setEditName(name);
  };

  const cancelEdit = () => {
    setEditingId(null);
    setEditName('');
  };

  const saveEdit = (collectionId: string, currentName: string) => {
    if (!editName.trim() || editName.trim() === currentName) {
      cancelEdit();
      return;
    }
    onRename(collectionId, editName.trim());
    setEditingId(null);
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full text-base-content/50">
        <Loader2 size={24} className="animate-spin" />
      </div>
    );
  }

  const modeLabel = safeMode ? t('tab.safe').toLowerCase() : t('tab.unsafe').toLowerCase();

  if (rows.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center p-8 text-center h-full absolute inset-0">
        <div className="w-16 h-16 rounded-full bg-base-300 flex items-center justify-center mb-4 text-base-content/30 mt-8">
          <Layers size={32} />
        </div>
        <h3 className="text-lg font-medium opacity-80 mb-2">
          {t('list.empty', { mode: modeLabel })}
        </h3>
        <p className="text-sm opacity-50 max-w-sm">{t('list.empty_desc', { mode: modeLabel })}</p>
      </div>
    );
  }

  return (
    <>
      <table className="table table-auto w-full">
        <thead className="sticky top-0 bg-base-200/95 backdrop-blur z-10 border-b border-base-content/5 shadow-sm">
          <tr className="border-none text-base-content/50">
            <th className="w-1/2">{t('list.table.name')}</th>
            <th>{t('list.table.mods')}</th>
            <th className="text-right pr-6">{t('list.table.actions')}</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => {
            const collection = row.kind === 'stored_collection' ? row.collection : null;
            const rowId = row.rowId;
            const isSelected = rowId === selectedId;
            const isEditing = collection ? editingId === collection.id : false;
            const isCurrentRuntime = row.kind === 'current_runtime';
            const isUnsaved = collection ? collection.is_unsaved : true;
            const isActive = collection ? collection.is_active : isCurrentRuntime;
            const label = collection
              ? getCollectionDisplayName({
                  name: collection.name,
                  isUnsaved: collection.is_unsaved,
                  isSafe: collection.is_safe,
                  labels: unsavedLabels,
                })
              : isCurrentRuntime
                ? row.label
                : '';
            const modCount = collection
              ? collection.mod_count
              : isCurrentRuntime
                ? row.modCount
                : 0;

            return (
              <tr
                key={rowId}
                onClick={() => onSelect(rowId)}
                className={`hover border-base-content/5 transition-colors group cursor-pointer ${
                  isSelected ? 'bg-primary/10 border-l-2 border-l-primary' : ''
                }`}
              >
                <td className="pl-4">
                  {isEditing && collection && !collection.is_unsaved ? (
                    <div className="flex items-center gap-2">
                      <input
                        type="text"
                        className="input input-sm input-bordered w-full max-w-30"
                        value={editName}
                        onChange={(e) => setEditName(e.target.value)}
                        onClick={(e) => e.stopPropagation()}
                        autoFocus
                        onKeyDown={(e) => {
                          if (e.key === 'Enter') saveEdit(collection.id, collection.name);
                          if (e.key === 'Escape') cancelEdit();
                        }}
                      />
                      <button
                        className="btn btn-xs btn-square btn-success text-success-content shrink-0"
                        onClick={(e) => {
                          e.stopPropagation();
                          saveEdit(collection.id, collection.name);
                        }}
                      >
                        <Check size={14} />
                      </button>
                      <button
                        className="btn btn-xs btn-square btn-ghost shrink-0"
                        onClick={(e) => {
                          e.stopPropagation();
                          cancelEdit();
                        }}
                      >
                        <X size={14} />
                      </button>
                    </div>
                  ) : (
                    <div className="font-medium text-[15px] flex items-center gap-2">
                      <span className="truncate max-w-30 2xl:max-w-50">{label}</span>
                      {isActive && (
                        <span className="badge badge-sm badge-success opacity-90 text-[10px] py-0 h-4 uppercase font-bold tracking-wider shrink-0">
                          {t('list.item.active', 'Active')}
                        </span>
                      )}
                      {isCurrentRuntime && (
                        <span className="badge badge-sm badge-ghost opacity-80 text-[10px] py-0 h-4 uppercase font-bold tracking-wider shrink-0">
                          {t('list.item.current_runtime', 'Live')}
                        </span>
                      )}
                      {collection && !collection.is_unsaved && (
                        <button
                          className="btn btn-xs btn-square btn-ghost opacity-0 group-hover:opacity-100 transition-opacity text-base-content/40 hover:text-base-content shrink-0"
                          onClick={(e) => {
                            e.stopPropagation();
                            startEdit(collection.id, collection.name);
                          }}
                          title={t('list.item.rename')}
                        >
                          <Edit2 size={12} />
                        </button>
                      )}
                    </div>
                  )}
                </td>
                <td>
                  <span className="badge badge-sm badge-ghost opacity-70 shrink-0">
                    {t('list.item.mod_count', { count: modCount })}
                  </span>
                </td>
                <td className="text-right pr-4">
                  <div className="flex items-center justify-end gap-2">
                    {isActive ? (
                      isUnsaved && onSave ? (
                        <button
                          className="btn btn-sm btn-secondary"
                          onClick={(e) => {
                            e.stopPropagation();
                            onSave({
                              mode: 'save_current_state',
                              sourceCollectionId: null,
                            });
                          }}
                        >
                          <Save size={14} />
                          {isCurrentRuntime
                            ? t('actions.save_current', 'Save')
                            : t('list.item.save_snapshot', 'Save')}
                        </button>
                      ) : null
                    ) : (
                      <button
                        className="btn btn-sm btn-primary"
                        onClick={(e) => {
                          e.stopPropagation();
                          if (!collection) {
                            return;
                          }
                          onApply(collection.id, collection.name);
                        }}
                        disabled={
                          isApplying || !collection || collection.mod_count === 0 || isActive
                        }
                      >
                        {isApplying ? (
                          <Loader2 size={14} className="animate-spin" />
                        ) : (
                          <>
                            <PlayCircle size={14} />
                            {t('list.item.apply', 'Apply')}
                          </>
                        )}
                      </button>
                    )}
                    {collection && !collection.is_unsaved && (
                      <button
                        className="btn btn-sm btn-square btn-ghost text-error/70 hover:text-error hover:bg-error/10 shrink-0"
                        onClick={(e) => {
                          e.stopPropagation();
                          setCollectionToDelete({ id: collection.id, name: collection.name });
                        }}
                        disabled={isDeleting}
                        title={t('list.item.delete')}
                      >
                        <Trash2 size={16} />
                      </button>
                    )}
                  </div>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
      <DeleteCollectionModal
        isOpen={collectionToDelete !== null}
        collectionName={collectionToDelete?.name ?? ''}
        isDeleting={isDeleting}
        onConfirm={() => {
          if (collectionToDelete) {
            onDelete(collectionToDelete.id);
            setCollectionToDelete(null);
          }
        }}
        onCancel={() => setCollectionToDelete(null)}
      />
    </>
  );
}
