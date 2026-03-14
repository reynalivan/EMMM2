import { useState } from 'react';
import { ChevronDown, ChevronRight, ShieldAlert } from 'lucide-react';
import { ModListRow } from './CollectionWorkspace';
import { type GroupedMod } from '../utils/groupMods';

interface ModGroupListProps {
  groups: GroupedMod[];
  colorClass: string;
}

export function ModGroupList({ groups, colorClass }: ModGroupListProps) {
  const [expanded, setExpanded] = useState<Set<string>>(new Set(groups.map((g) => g.id)));

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
      <div className="text-center p-4 text-sm text-base-content/40 border border-white/5 border-dashed rounded-lg bg-base-100/10">
        No mods in this state
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
            className="border border-white/5 rounded-lg overflow-hidden bg-base-100/30"
          >
            <button
              onClick={() => toggle(obj.id)}
              className="w-full flex items-center justify-between px-3 py-2 hover:bg-base-300/30 transition-colors"
            >
              <div className="flex items-center gap-2">
                <div className="text-base-content/50">
                  {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                </div>
                <span className="font-semibold text-xs text-left">{obj.name}</span>
                <span className="text-[9px] text-base-content/40 uppercase tracking-widest hidden sm:inline-block">
                  {obj.type}
                </span>
              </div>
              <div className="flex items-center gap-2 shrink-0">
                <span className={`text-[10px] ${colorClass}`}>{obj.mods.length} mods</span>
                {obj.unsafeCount > 0 && (
                  <span className="flex items-center gap-0.5 text-[10px] text-error/70">
                    <ShieldAlert size={10} />
                    {obj.unsafeCount}
                  </span>
                )}
              </div>
            </button>
            {isExpanded && (
              <div className="border-t border-white/5 py-1 px-1">
                {obj.mods.map((mod) => (
                  <ModListRow key={mod.id} mod={mod} />
                ))}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
