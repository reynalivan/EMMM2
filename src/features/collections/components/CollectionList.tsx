/**
 * CollectionList — Left panel showing all collections for the current corridor.
 *
 * Extracted from CollectionsPage. Replaces the inline table + workspaceRows chain.
 * Uses v2 types directly — no intermediary transformation.
 */

import { Layers, Trash2, Edit2, Check, X, PlayCircle, Loader2, Save } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { CollectionSummary } from '../../../types/collection';

interface CollectionListProps {
  collections: CollectionSummary[];
  selectedId: string | null;
  isLoading: boolean;
  safeMode: boolean;
  onSelect: (id: string) => void;
  onApply: (id: string, name: string) => void;
  onDelete: (id: string) => void;
  onRename: (id: string, newName: string) => void;
  onSave?: () => void;
  isApplying: boolean;
  isDeleting: boolean;
}

export function CollectionList({
  collections,
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
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState('');

  const startEdit = (c: CollectionSummary) => {
    setEditingId(c.id);
    setEditName(c.name);
  };

  const cancelEdit = () => {
    setEditingId(null);
    setEditName('');
  };

  const saveEdit = (c: CollectionSummary) => {
    if (!editName.trim() || editName.trim() === c.name) {
      cancelEdit();
      return;
    }
    onRename(c.id, editName.trim());
    setEditingId(null);
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full text-base-content/50">
        <Loader2 size={24} className="animate-spin" />
      </div>
    );
  }

  // Hide undo targets from the list. Allow unsaved and named.
  const visibleCollections = collections.filter((c) => !c.is_undo_target);
  const modeLabel = safeMode ? t('tab.safe').toLowerCase() : t('tab.unsafe').toLowerCase();

  if (visibleCollections.length === 0) {
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
    <table className="table table-auto w-full">
      <thead className="sticky top-0 bg-base-200/95 backdrop-blur z-10 border-b border-base-content/5 shadow-sm">
        <tr className="border-none text-base-content/50">
          <th className="w-1/2">{t('list.table.name')}</th>
          <th>{t('list.table.mods')}</th>
          <th className="text-right pr-6">{t('list.table.actions')}</th>
        </tr>
      </thead>
      <tbody>
        {visibleCollections.map((c) => {
          const isSelected = c.id === selectedId;
          const isEditing = editingId === c.id;

          return (
            <tr
              key={c.id}
              onClick={() => onSelect(c.id)}
              className={`hover border-base-content/5 transition-colors group cursor-pointer ${
                isSelected ? 'bg-primary/10 border-l-2 border-l-primary' : ''
              }`}
            >
              <td className="pl-4">
                {isEditing && !c.is_unsaved ? (
                  <div className="flex items-center gap-2">
                    <input
                      type="text"
                      className="input input-sm input-bordered w-full max-w-30"
                      value={editName}
                      onChange={(e) => setEditName(e.target.value)}
                      onClick={(e) => e.stopPropagation()}
                      autoFocus
                      onKeyDown={(e) => {
                        if (e.key === 'Enter') saveEdit(c);
                        if (e.key === 'Escape') cancelEdit();
                      }}
                    />
                    <button
                      className="btn btn-xs btn-square btn-success text-success-content shrink-0"
                      onClick={(e) => {
                        e.stopPropagation();
                        saveEdit(c);
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
                    <span className="truncate max-w-30 2xl:max-w-50">
                      {c.is_unsaved ? t('context.unsaved', 'Unsaved Preset') : c.name}
                    </span>
                    {c.is_active && (
                      <span className="badge badge-sm badge-success opacity-90 text-[10px] py-0 h-4 uppercase font-bold tracking-wider shrink-0">
                        {t('list.item.active', 'Active')}
                      </span>
                    )}
                    {!c.is_unsaved && (
                      <button
                        className="btn btn-xs btn-square btn-ghost opacity-0 group-hover:opacity-100 transition-opacity text-base-content/40 hover:text-base-content shrink-0"
                        onClick={(e) => {
                          e.stopPropagation();
                          startEdit(c);
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
                  {t('list.item.mod_count', { count: c.member_count })}
                </span>
              </td>
              <td className="text-right pr-4">
                <div className="flex items-center justify-end gap-2">
                  {c.is_active ? (
                    c.is_unsaved && onSave ? (
                      <button
                        className="btn btn-sm btn-secondary"
                        onClick={(e) => {
                          e.stopPropagation();
                          onSave();
                        }}
                      >
                        <Save size={14} />
                        {t('actions.save_current', 'Save')}
                      </button>
                    ) : null
                  ) : (
                    <button
                      className="btn btn-sm btn-primary"
                      onClick={(e) => {
                        e.stopPropagation();
                        onApply(c.id, c.name);
                      }}
                      disabled={isApplying || c.member_count === 0}
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
                  {!c.is_unsaved && (
                    <button
                      className="btn btn-sm btn-square btn-ghost text-error/70 hover:text-error hover:bg-error/10 shrink-0"
                      onClick={(e) => {
                        e.stopPropagation();
                        onDelete(c.id);
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
  );
}
