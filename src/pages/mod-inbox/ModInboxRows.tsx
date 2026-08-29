import { Archive, Boxes, ExternalLink, FolderOpen, PackageOpen } from 'lucide-react';
import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { formatBytes } from '../../shared/lib/utils/formatters';
import type {
  ModInboxEntry,
  ModInboxEntryLayout,
  ProcessedModInboxDestination,
  ProcessedModInboxSource,
} from './types';
import { parseModInboxTimestamp } from './time';

export function ReadyEntryRow({
  entry,
  selected,
  onToggle,
  onResume,
}: {
  entry: ModInboxEntry;
  selected: boolean;
  onToggle: () => void;
  onResume: () => void;
}) {
  const { t } = useTranslation('mod_inbox');
  const layoutLabels: Record<ModInboxEntryLayout, string> = {
    direct_mod: t('ready.layouts.direct_mod'),
    folder_pack: t('ready.layouts.folder_pack', { count: entry.detectedRootCount }),
    wrapper: t('ready.layouts.wrapper'),
    unknown: t('ready.layouts.unknown'),
  };

  return (
    <article className="flex items-center gap-4 rounded-2xl border border-base-300 bg-base-200/45 p-4">
      <input
        type="checkbox"
        className="checkbox checkbox-sm"
        aria-label={t('ready.select_entry', { name: entry.name })}
        checked={selected}
        disabled={Boolean(entry.pendingBatchId)}
        onChange={onToggle}
      />
      <div className="rounded-xl bg-base-300/70 p-2.5 text-primary">
        {entry.kind === 'archive' ? <Archive size={20} /> : <FolderOpen size={20} />}
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <h3 className="truncate font-semibold">{entry.name}</h3>
          <span className={`badge badge-sm ${entry.layout === 'unknown' ? 'badge-warning' : ''}`}>
            {layoutLabels[entry.layout]}
          </span>
          {entry.archiveFormat && (
            <span className="badge badge-ghost badge-sm uppercase">{entry.archiveFormat}</span>
          )}
        </div>
        <p className="mt-1 truncate text-xs text-base-content/45">{entry.path}</p>
        <p className="mt-1 text-[11px] text-base-content/50">
          {entry.sizeBytes != null ? formatBytes(entry.sizeBytes) : t('ready.size_unknown')}
          {' · '}
          {new Date(Number(entry.modifiedUnixMs)).toLocaleString()}
        </p>
      </div>
      {entry.pendingBatchId && (
        <button type="button" className="btn btn-outline btn-sm" onClick={onResume}>
          {t('ready.resume_review')}
        </button>
      )}
    </article>
  );
}

export function ProcessedSourceRow({
  source,
  selected,
  onToggle,
  onOpenInApp,
  onOpenInExplorer,
}: {
  source: ProcessedModInboxSource;
  selected: boolean;
  onToggle: () => void;
  onOpenInApp: (destination: ProcessedModInboxDestination) => void;
  onOpenInExplorer: (destination: ProcessedModInboxDestination) => void;
}) {
  const { t } = useTranslation('mod_inbox');
  const sourceRetained = Boolean(source.processedPath) && !source.sourceDeletedAt;
  const sourceStatus = sourceRetained
    ? t('processed.source_retained')
    : source.sourceDeletedAt
      ? t('processed.source_deleted')
      : t('processed.source_moved');

  return (
    <article className="rounded-2xl border border-base-300 bg-base-200/45 p-4">
      <div className="flex items-start gap-4">
        <input
          type="checkbox"
          className="checkbox checkbox-sm mt-1"
          aria-label={t('processed.select_source', { name: source.name })}
          checked={selected}
          disabled={!sourceRetained}
          onChange={onToggle}
        />
        <div className="rounded-xl bg-base-300/70 p-2.5 text-secondary">
          {source.sourceKind === 'archive' ? <Archive size={20} /> : <FolderOpen size={20} />}
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <h3 className="font-semibold">{source.name}</h3>
            <span className={`badge badge-sm ${sourceRetained ? 'badge-success' : 'badge-ghost'}`}>
              {sourceStatus}
            </span>
          </div>
          <p className="mt-1 truncate text-xs text-base-content/45">{source.originalPath}</p>
          <p className="mt-1 text-[11px] text-base-content/50">
            {parseModInboxTimestamp(source.processedAt).toLocaleString()}
          </p>
        </div>
      </div>

      <div className="mt-4 space-y-2 border-l border-base-300 pl-5 sm:ml-11">
        {source.destinations.length === 0 ? (
          <p className="py-2 text-sm text-base-content/45">{t('processed.no_destinations')}</p>
        ) : (
          source.destinations.map((destination) => (
            <div
              key={`${destination.objectId}:${destination.placedPath}`}
              className="flex flex-wrap items-center gap-3 rounded-xl bg-base-100/70 p-3"
            >
              <Boxes size={17} className="text-primary" />
              <div className="min-w-0 flex-1">
                <p className="text-sm font-medium">
                  {destination.objectName ?? t('processed.destination_unavailable')}
                </p>
                <p className="truncate text-xs text-base-content/50">{destination.plannedName}</p>
              </div>
              <span className="badge badge-outline badge-sm">{destination.status}</span>
              <button
                type="button"
                className="btn btn-ghost btn-xs gap-1"
                aria-label={t('processed.open_in_app_label', {
                  name: destination.objectName ?? t('processed.destination_label'),
                })}
                disabled={!destination.objectId}
                onClick={() => onOpenInApp(destination)}
              >
                <PackageOpen size={14} /> {t('processed.open_in_app')}
              </button>
              <button
                type="button"
                className="btn btn-ghost btn-xs gap-1"
                aria-label={t('processed.open_in_explorer_label', {
                  name: destination.objectName ?? t('processed.destination_label'),
                })}
                onClick={() => onOpenInExplorer(destination)}
              >
                <ExternalLink size={14} /> {t('processed.open_in_explorer')}
              </button>
            </div>
          ))
        )}
      </div>
    </article>
  );
}

export function EmptyState({
  icon,
  title,
  description,
}: {
  icon: ReactNode;
  title: string;
  description: string;
}) {
  return (
    <div className="rounded-3xl border border-dashed border-base-300 py-16 text-center text-base-content/50">
      <div className="mx-auto mb-3 w-fit">{icon}</div>
      <h2 className="font-semibold text-base-content/80">{title}</h2>
      <p className="mx-auto mt-1 max-w-md text-sm">{description}</p>
    </div>
  );
}
