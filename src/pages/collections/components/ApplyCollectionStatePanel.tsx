import { useState } from 'react';
import type { TFunction } from 'i18next';
import type { PreviewTreeNode } from '@/entities/collection';
import type { ApplyPreviewChange } from '../applyPreviewDiff';
import { CollectionTreeView } from './CollectionTreeView';
export function SummaryStat({ label, value }: { label: string; value: number }) {
  return (
    <div className="rounded-xl border border-base-content/8 bg-base-300/20 px-3 py-2">
      <div className="text-[10px] uppercase tracking-[0.18em] text-base-content/45">{label}</div>
      <div className="mt-1 text-sm font-semibold text-base-content/85">{value}</div>
    </div>
  );
}

interface StatePanelProps {
  containerClass: string;
  heading: string;
  title: string;
  titleClass: string;
  summary: { active_root_count: number; enabled_object_count: number; object_count: number };
  nodes?: PreviewTreeNode[];
  colorClass: string;
  emptyMessage: string;
  treeIdentity: string;
  nodeChanges?: ReadonlyMap<string, ApplyPreviewChange>;
  t: TFunction;
}

/** One side of the before/after comparison. Both sides render identically. */
export function StatePanel({
  containerClass,
  heading,
  title,
  titleClass,
  summary,
  nodes,
  colorClass,
  emptyMessage,
  treeIdentity,
  nodeChanges,
  t,
}: StatePanelProps) {
  const [treeScrollElement, setTreeScrollElement] = useState<HTMLDivElement | null>(null);
  return (
    <div className={`flex-1 flex flex-col max-h-full overflow-hidden ${containerClass}`}>
      <div className="p-4 bg-base-300/30 border-b border-base-content/5 shrink-0">
        <div className="flex items-center justify-between gap-4">
          <div>
            <div className="text-[11px] uppercase tracking-[0.18em] text-base-content/45">
              {heading}
            </div>
            <div className={`mt-1 text-lg font-semibold ${titleClass}`}>{title}</div>
          </div>
          <SummaryStat
            label={t('collections:apply.summary.mods', 'Active Roots')}
            value={summary.active_root_count}
          />
        </div>
      </div>
      <div className="p-4 grid grid-cols-2 gap-3 border-b border-base-content/5 bg-base-100/30">
        <SummaryStat
          label={t('collections:apply.summary.objects_on', 'Objects On')}
          value={summary.enabled_object_count}
        />
        <SummaryStat
          label={t('collections:apply.summary.objects', 'Objects')}
          value={summary.object_count}
        />
      </div>
      <div ref={setTreeScrollElement} className="flex-1 overflow-y-auto custom-scrollbar p-4">
        <CollectionTreeView
          nodes={nodes}
          colorClass={colorClass}
          emptyMessage={emptyMessage}
          scrollElement={treeScrollElement}
          treeIdentity={treeIdentity}
          nodeChanges={nodeChanges}
        />
      </div>
    </div>
  );
}
