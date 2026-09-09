import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { ChevronDown, Image, Search } from 'lucide-react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import type {
  ConfidenceTier,
  DestinationMatchMethod,
  DestinationSuggestion,
  ImportBatch,
  ImportDecision,
  ImportItem,
} from '../../../shared/api/tauri/bindings.gen';
import type { ObjectSummary } from '@/entities/game-object';
import { destinationDecision } from '../utils/importBatchDecision';

type Props = {
  batch: ImportBatch;
  item: ImportItem;
  objects: ObjectSummary[];
  busy: boolean;
  onChooseDestination: (
    item: ImportItem,
    suggestion: DestinationSuggestion,
    decision: ImportDecision,
  ) => Promise<void>;
  onChooseManualTarget: (item: ImportItem, objectId: string) => Promise<void>;
};

type PopoverPosition = {
  left: number;
  top: number;
  width: number;
  maxHeight: number;
};

export function ImportBatchWizardDestinationPanel({
  batch,
  busy,
  item,
  objects,
  onChooseDestination,
  onChooseManualTarget,
}: Props) {
  const { t } = useTranslation('match_wizard');
  const containerRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [popoverPosition, setPopoverPosition] = useState<PopoverPosition | null>(null);
  const objectById = useMemo(
    () => new Map(objects.map((object) => [object.id, object])),
    [objects],
  );
  const suggestions = useMemo(
    () =>
      [...item.destinationSuggestions].sort(
        (a, b) => b.confidencePercentage - a.confidencePercentage,
      ),
    [item.destinationSuggestions],
  );
  const proposed = suggestions[0] ?? null;
  const selectedObject = item.destinationObjectId
    ? (objectById.get(item.destinationObjectId) ?? null)
    : proposed?.objectId
      ? (objectById.get(proposed.objectId) ?? null)
      : null;
  const selectedName = selectedObject?.name ?? proposed?.folderName ?? null;
  const suggestionByObjectId = new Map(
    suggestions.flatMap((suggestion) =>
      suggestion.objectId ? [[suggestion.objectId, suggestion] as const] : [],
    ),
  );
  const normalizedQuery = query.trim().toLocaleLowerCase();
  const allObjects = objects
    .filter((object) => {
      if (!normalizedQuery) return true;
      return [object.name, object.object_type, object.tags]
        .join(' ')
        .toLocaleLowerCase()
        .includes(normalizedQuery);
    })
    .sort((a, b) => {
      const confidenceDifference =
        (suggestionByObjectId.get(b.id)?.confidencePercentage ?? 0) -
        (suggestionByObjectId.get(a.id)?.confidencePercentage ?? 0);
      return confidenceDifference || a.name.localeCompare(b.name);
    })
    .slice(0, 50);
  const potentialSuggestions = suggestions.filter(
    (suggestion) => suggestion.kind !== 'existing_object' || suggestion.confidencePercentage >= 45,
  );
  const filteredSuggestions = potentialSuggestions.filter((suggestion) => {
    if (!normalizedQuery) return true;
    const object = suggestion.objectId ? objectById.get(suggestion.objectId) : null;
    return [suggestion.folderName, object?.name, object?.object_type]
      .filter(Boolean)
      .join(' ')
      .toLocaleLowerCase()
      .includes(normalizedQuery);
  });

  const updatePopoverPosition = useCallback(() => {
    const trigger = triggerRef.current;
    if (!trigger) return;
    const rect = trigger.getBoundingClientRect();
    const viewportMargin = 8;
    const gap = 6;
    const width = Math.min(384, window.innerWidth - viewportMargin * 2);
    const left = Math.min(
      Math.max(viewportMargin, rect.left),
      window.innerWidth - width - viewportMargin,
    );
    const below = window.innerHeight - rect.bottom - viewportMargin - gap;
    const above = rect.top - viewportMargin - gap;
    const openAbove = below < 280 && above > below;
    const availableHeight = openAbove ? above : below;
    const maxHeight = Math.max(160, Math.min(384, availableHeight));
    const top = openAbove ? rect.top - maxHeight - gap : rect.bottom + gap;
    setPopoverPosition({ left, top, width, maxHeight });
  }, []);

  useEffect(() => {
    if (!open) return;
    const closeOutside = (event: MouseEvent) => {
      const target = event.target as Node;
      if (!containerRef.current?.contains(target) && !popoverRef.current?.contains(target)) {
        setOpen(false);
      }
    };
    updatePopoverPosition();
    document.addEventListener('mousedown', closeOutside);
    window.addEventListener('resize', updatePopoverPosition);
    window.addEventListener('scroll', updatePopoverPosition, true);
    return () => {
      document.removeEventListener('mousedown', closeOutside);
      window.removeEventListener('resize', updatePopoverPosition);
      window.removeEventListener('scroll', updatePopoverPosition, true);
    };
  }, [open, updatePopoverPosition]);

  const chooseSuggestion = async (suggestion: DestinationSuggestion) => {
    await onChooseDestination(item, suggestion, destinationDecision(batch, suggestion));
    setOpen(false);
    setQuery('');
  };

  const chooseObject = async (objectId: string) => {
    await onChooseManualTarget(item, objectId);
    setOpen(false);
    setQuery('');
  };

  return (
    <div ref={containerRef} className="min-w-0">
      <button
        ref={triggerRef}
        type="button"
        className="flex h-11 w-full items-center gap-2 rounded-lg border border-base-300 bg-base-100 px-2 text-left hover:border-primary"
        disabled={busy || !['awaiting_destination', 'ready', 'skipped'].includes(item.status)}
        onClick={() => setOpen((current) => !current)}
        aria-expanded={open}
      >
        <DestinationAvatar object={selectedObject} />
        <span className="min-w-0 flex-1">
          <span className="block truncate text-sm font-semibold">
            {selectedName ?? t('no_destination')}
          </span>
          <span className="block truncate text-[11px] text-base-content/50">
            {selectedObject?.object_type ??
              (proposed
                ? t(`match_methods.${effectiveMatchMethod(proposed)}`)
                : t('destination.search_hint'))}
          </span>
        </span>
        <ChevronDown size={15} className="shrink-0 text-base-content/45" aria-hidden="true" />
      </button>

      {open &&
        popoverPosition &&
        createPortal(
          <div
            ref={popoverRef}
            className="fixed z-[1000] overflow-hidden rounded-xl border border-base-300 bg-base-100 shadow-2xl"
            style={popoverPosition}
          >
            <label className="relative block border-b border-base-300 p-2">
              <Search
                size={14}
                className="absolute left-5 top-1/2 -translate-y-1/2 text-base-content/40"
              />
              <input
                className="input input-sm input-bordered w-full pl-8"
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder={t('destination.search_placeholder')}
                autoFocus
              />
            </label>
            <div
              className="overflow-y-auto px-1.5 pb-1.5"
              style={{ maxHeight: Math.max(96, popoverPosition.maxHeight - 53) }}
            >
              {filteredSuggestions.length > 0 && (
                <DestinationGroupLabel>{t('destination.potential')}</DestinationGroupLabel>
              )}
              {filteredSuggestions.map((suggestion) => {
                const object = suggestion.objectId
                  ? (objectById.get(suggestion.objectId) ?? null)
                  : null;
                return (
                  <button
                    key={`${suggestion.kind}:${suggestion.objectId ?? suggestion.canonicalEntryKey}`}
                    type="button"
                    className="flex w-full items-center gap-2 rounded-lg px-2 py-2 text-left hover:bg-base-200"
                    onClick={() => void chooseSuggestion(suggestion)}
                  >
                    <DestinationAvatar object={object} />
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-sm font-semibold">
                        {object?.name ?? suggestion.folderName}
                      </span>
                      <span className="block truncate text-[11px] text-base-content/50">
                        {object?.object_type ??
                          t(`match_methods.${effectiveMatchMethod(suggestion)}`)}
                      </span>
                    </span>
                    <span
                      className={`badge badge-sm tabular-nums ${confidenceBadgeClass(
                        suggestion.confidenceTier,
                      )}`}
                    >
                      {suggestion.confidencePercentage}%
                    </span>
                  </button>
                );
              })}

              {allObjects.length > 0 && (
                <DestinationGroupLabel>{t('destination.all')}</DestinationGroupLabel>
              )}
              {allObjects.map((object) => {
                const suggestion = suggestionByObjectId.get(object.id);
                return (
                  <button
                    key={object.id}
                    type="button"
                    className="flex w-full items-center gap-2 rounded-lg px-2 py-2 text-left hover:bg-base-200"
                    onClick={() => void chooseObject(object.id)}
                  >
                    <DestinationAvatar object={object} />
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-sm font-medium">{object.name}</span>
                      <span className="block truncate text-[11px] text-base-content/50">
                        {object.object_type} ·{' '}
                        {t(`match_methods.${effectiveMatchMethod(suggestion)}`)}
                      </span>
                    </span>
                    <span
                      className={`badge badge-sm tabular-nums ${confidenceBadgeClass(
                        suggestion?.confidenceTier ?? 'no_match',
                      )}`}
                    >
                      {suggestion?.confidencePercentage ?? 0}%
                    </span>
                  </button>
                );
              })}
              {filteredSuggestions.length === 0 && allObjects.length === 0 && (
                <p className="px-3 py-6 text-center text-sm text-base-content/50">
                  {t('destination.no_results')}
                </p>
              )}
            </div>
          </div>,
          document.body,
        )}
    </div>
  );
}

function DestinationGroupLabel({ children }: { children: string }) {
  return (
    <p className="sticky -top-px z-10 -mx-1.5 bg-base-100 px-3 py-2 text-[10px] font-bold uppercase tracking-wider text-base-content/45 shadow-[0_1px_0_hsl(var(--bc)/0.08)]">
      {children}
    </p>
  );
}

function confidenceBadgeClass(tier: ConfidenceTier): string {
  if (tier === 'high') return 'badge-success';
  if (tier === 'medium') return 'badge-warning';
  if (tier === 'low') return 'badge-info';
  return 'badge-ghost';
}

function effectiveMatchMethod(
  suggestion: DestinationSuggestion | undefined,
): DestinationMatchMethod {
  if (!suggestion) return 'no_name_match';
  if (
    suggestion.matchMethod === 'no_name_match' &&
    (suggestion.kind === 'create_canonical' ||
      (suggestion.canonicalEntryKey !== null && suggestion.confidencePercentage >= 75))
  ) {
    return 'canonical_identity';
  }
  return suggestion.matchMethod ?? 'no_name_match';
}

function DestinationAvatar({ object }: { object: ObjectSummary | null }) {
  if (object?.thumbnail_path) {
    return (
      <img
        src={convertFileSrc(object.thumbnail_path)}
        alt=""
        className="h-8 w-8 shrink-0 rounded-lg bg-base-300 object-cover"
      />
    );
  }
  return (
    <span className="grid h-8 w-8 shrink-0 place-items-center rounded-lg bg-base-300 text-base-content/35">
      <Image size={15} aria-hidden="true" />
    </span>
  );
}
