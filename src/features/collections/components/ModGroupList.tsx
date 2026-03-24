import { useEffect, useMemo, useState } from 'react';
import { ChevronDown, ChevronRight, ShieldAlert, EyeOff } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { CollectionModRow } from './CollectionModRow';
import { type GroupedMod } from '../utils/groupMods';

interface ModGroupListProps {
  groups: GroupedMod[];
  colorClass: string;
  emptyGroupMessage?: string;
  emptyStateMessage?: string;
  onToggleObjectState?: (objectId: string) => void;
  resetKey?: string;
  expansionMode?: 'default' | 'all' | 'none';
}

export function ModGroupList({
  groups,
  colorClass,
  emptyGroupMessage,
  emptyStateMessage,
  onToggleObjectState,
  resetKey,
  expansionMode,
}: ModGroupListProps) {
  const { t } = useTranslation('collections');
  const resolvedEmptyGroupMessage = emptyGroupMessage ?? t('list.item.mod_count', { count: 0 });
  const resolvedEmptyStateMessage = emptyStateMessage ?? t('preview.empty');
  const groupIdsKey = useMemo(() => groups.map((group) => group.id).join('|'), [groups]);
  const defaultExpanded = useMemo(() => new Set(groups.map((group) => group.id)), [groups]);
  const [expanded, setExpanded] = useState<Set<string>>(defaultExpanded);

  useEffect(() => {
    if (expansionMode === 'all' || expansionMode === 'default' || expansionMode === undefined) {
      setExpanded(defaultExpanded);
      return;
    }

    if (expansionMode === 'none') {
      setExpanded(new Set());
    }
  }, [defaultExpanded, expansionMode, groupIdsKey, resetKey]);

  const toggle = (id: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  if (groups.length === 0) {
    return (
      <div className="text-center p-4 text-sm text-base-content/40 border border-base-content/5 border-dashed rounded-lg bg-base-content/5">
        {resolvedEmptyStateMessage}
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {groups.map((obj) => {
        const isExpanded = expanded.has(obj.id);
        return (
          <div
            key={obj.id}
            className="border border-base-content/5 rounded-lg overflow-hidden bg-base-content/5"
          >
            <div className="w-full flex items-center justify-between px-3 py-2 hover:bg-base-300/30 transition-colors">
              <button
                onClick={() => toggle(obj.id)}
                className="flex items-center gap-2 min-w-0 flex-1 text-left"
              >
                <div className="text-base-content/50">
                  {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                </div>
                <span className="font-semibold text-xs text-left truncate">{obj.name}</span>
                <span className="text-[9px] text-base-content/40 uppercase tracking-widest hidden sm:inline-block shrink-0">
                  {obj.type}
                </span>
              </button>
              <div className="flex items-center gap-2 shrink-0">
                {typeof obj.is_enabled === 'boolean' && onToggleObjectState && (
                  <input
                    type="checkbox"
                    className="toggle toggle-xs toggle-primary"
                    checked={obj.is_enabled}
                    disabled={false}
                    onChange={() => onToggleObjectState(obj.id)}
                  />
                )}
                {typeof obj.is_enabled === 'boolean' && !onToggleObjectState && (
                  <span
                    className={`badge badge-xs ${obj.is_enabled ? 'badge-success' : 'badge-neutral'} gap-1`}
                  >
                    {!obj.is_enabled && <EyeOff size={10} />}
                    {obj.is_enabled ? t('common:status.enabled') : t('common:status.disabled')}
                  </span>
                )}
                <span className={`text-[10px] ${colorClass}`}>
                  {t('list.item.mod_count', { count: obj.mods.length })}
                </span>
                {obj.unsafeCount > 0 && (
                  <span className="flex items-center gap-0.5 text-[10px] text-error/70">
                    <ShieldAlert size={10} />
                    {obj.unsafeCount}
                  </span>
                )}
              </div>
            </div>
            {isExpanded && (
              <div className="border-t border-base-content/5 py-1 px-1">
                {obj.mods.length > 0 ? (
                  obj.mods.map((mod) => <CollectionModRow key={mod.path_key} mod={mod} />)
                ) : (
                  <div className="px-3 py-3 text-xs text-base-content/45 italic">
                    {resolvedEmptyGroupMessage}
                  </div>
                )}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
