import { useCallback, useEffect, useId, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { ImageIcon } from 'lucide-react';
import type {
  CanonicalClassificationCatalogEntry,
  CanonicalSuggestion,
} from '../../../shared/api/tauri/bindings.gen';
import { LiquidSurface } from '@/shared/ui/liquid';
import { getFileUrl } from '@/shared/lib/utils';

type CanonicalObjectComboboxProps = {
  ariaLabel?: string;
  categoryLabel?: (category: string) => string;
  disabled?: boolean;
  emptyLabel: string;
  entries: CanonicalClassificationCatalogEntry[];
  manualOptionHint?: string;
  manualOptionLabel?: string;
  onSelectManual?: () => void;
  searchPlaceholder?: string;
  suggestions: CanonicalSuggestion[];
  selectedEntryKey: string | null;
  onSelect: (entryKey: string) => void;
};

const MAX_VISIBLE_RESULTS = 24;

type PopoverPosition = {
  left: number;
  top: number;
  width: number;
  maxHeight: number;
};

function searchRank(query: string, entry: CanonicalClassificationCatalogEntry): number | null {
  const normalizedQuery = query.trim().toLocaleLowerCase();
  if (!normalizedQuery) return 0;
  const name = entry.name.toLocaleLowerCase();
  if (name === normalizedQuery) return 0;
  if (name.startsWith(normalizedQuery)) return 1;
  if (name.includes(normalizedQuery)) return 2;
  if (entry.aliases.some((alias) => alias.toLocaleLowerCase() === normalizedQuery)) return 3;
  if (entry.aliases.some((alias) => alias.toLocaleLowerCase().includes(normalizedQuery))) return 4;
  return null;
}

function metadataChips(entry: CanonicalClassificationCatalogEntry): string[] {
  if (!entry.metadata || Array.isArray(entry.metadata) || typeof entry.metadata !== 'object') {
    return [];
  }
  return Object.entries(entry.metadata)
    .filter(([, value]) => ['string', 'number', 'boolean'].includes(typeof value))
    .slice(0, 2)
    .map(([key, value]) => `${key}: ${String(value)}`);
}

export function CanonicalObjectCombobox({
  ariaLabel = 'Canonical object',
  categoryLabel = (category) => category,
  disabled = false,
  emptyLabel,
  entries,
  manualOptionHint,
  manualOptionLabel,
  onSelectManual,
  searchPlaceholder = 'Search canonical object',
  suggestions,
  selectedEntryKey,
  onSelect,
}: CanonicalObjectComboboxProps) {
  const listId = useId();
  const [isOpen, setIsOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [activeIndex, setActiveIndex] = useState(0);
  const triggerRef = useRef<HTMLInputElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const [popoverPosition, setPopoverPosition] = useState<PopoverPosition | null>(null);
  const selected = entries.find((entry) => entry.entryKey === selectedEntryKey) ?? null;
  const suggestedKeys = useMemo(
    () => new Set(suggestions.map((suggestion) => suggestion.entryKey)),
    [suggestions],
  );
  const visibleEntries = useMemo(() => {
    const ranks = entries
      .map((entry) => ({ entry, rank: searchRank(query, entry) }))
      .filter(
        (candidate): candidate is { entry: CanonicalClassificationCatalogEntry; rank: number } =>
          candidate.rank !== null,
      )
      .sort(
        (left, right) =>
          left.rank - right.rank ||
          Number(suggestedKeys.has(right.entry.entryKey)) -
            Number(suggestedKeys.has(left.entry.entryKey)) ||
          left.entry.name.localeCompare(right.entry.name),
      );
    if (query.trim()) {
      return ranks.slice(0, MAX_VISIBLE_RESULTS).map((candidate) => candidate.entry);
    }

    const suggested = suggestions
      .map((suggestion) => entries.find((entry) => entry.entryKey === suggestion.entryKey))
      .filter((entry): entry is CanonicalClassificationCatalogEntry => entry !== undefined);
    const suggestedKeySet = new Set(suggested.map((entry) => entry.entryKey));
    return [...suggested, ...entries.filter((entry) => !suggestedKeySet.has(entry.entryKey))].slice(
      0,
      MAX_VISIBLE_RESULTS,
    );
  }, [entries, query, suggestedKeys, suggestions]);
  const activeEntry = visibleEntries[activeIndex] ?? null;
  const hasManualOption = onSelectManual !== undefined && manualOptionLabel !== undefined;
  const activeManualOption = hasManualOption && activeIndex === visibleEntries.length;
  const optionCount = visibleEntries.length + Number(hasManualOption);

  const selectEntry = (entryKey: string) => {
    onSelect(entryKey);
    setQuery('');
    setActiveIndex(0);
    setIsOpen(false);
  };

  const selectManual = () => {
    onSelectManual?.();
    setQuery('');
    setActiveIndex(0);
    setIsOpen(false);
  };

  const updatePopoverPosition = useCallback(() => {
    const trigger = triggerRef.current;
    if (!trigger) return;
    const rect = trigger.getBoundingClientRect();
    const gutter = 8;
    const width = Math.min(Math.max(rect.width, 320), window.innerWidth - gutter * 2);
    const left = Math.min(Math.max(gutter, rect.left), window.innerWidth - width - gutter);
    const below = window.innerHeight - rect.bottom - gutter - 6;
    const above = rect.top - gutter - 6;
    const opensAbove = below < 280 && above > below;
    const maxHeight = Math.max(144, Math.min(360, opensAbove ? above : below));
    setPopoverPosition({
      left,
      top: opensAbove ? rect.top - maxHeight - 6 : rect.bottom + 6,
      width,
      maxHeight,
    });
  }, []);

  useLayoutEffect(() => {
    if (!isOpen) return;
    updatePopoverPosition();
    window.addEventListener('resize', updatePopoverPosition);
    window.addEventListener('scroll', updatePopoverPosition, true);
    return () => {
      window.removeEventListener('resize', updatePopoverPosition);
      window.removeEventListener('scroll', updatePopoverPosition, true);
    };
  }, [isOpen, updatePopoverPosition]);

  useEffect(() => {
    if (!isOpen) return;
    const closeOutside = (event: PointerEvent) => {
      const target = event.target as Node;
      if (triggerRef.current?.contains(target) || popoverRef.current?.contains(target)) return;
      setIsOpen(false);
    };
    document.addEventListener('pointerdown', closeOutside);
    return () => document.removeEventListener('pointerdown', closeOutside);
  }, [isOpen]);

  return (
    <div className="relative min-w-72">
      <input
        ref={triggerRef}
        aria-activedescendant={
          isOpen && activeEntry
            ? `${listId}-${activeEntry.entryKey}`
            : activeManualOption
              ? `${listId}-manual`
              : undefined
        }
        aria-autocomplete="list"
        aria-controls={listId}
        aria-expanded={isOpen}
        aria-haspopup="listbox"
        aria-label={ariaLabel}
        className="input input-bordered input-sm w-full"
        disabled={disabled}
        placeholder={selected?.name ?? searchPlaceholder}
        role="combobox"
        value={isOpen ? query : (selected?.name ?? '')}
        onBlur={() => setIsOpen(false)}
        onChange={(event) => {
          setQuery(event.target.value);
          setActiveIndex(0);
          setIsOpen(true);
        }}
        onFocus={() => {
          setQuery('');
          setActiveIndex(0);
          setIsOpen(true);
        }}
        onKeyDown={(event) => {
          if (event.key === 'ArrowDown') {
            event.preventDefault();
            setIsOpen(true);
            setActiveIndex((current) => Math.min(current + 1, Math.max(optionCount - 1, 0)));
          } else if (event.key === 'ArrowUp') {
            event.preventDefault();
            setIsOpen(true);
            setActiveIndex((current) => Math.max(current - 1, 0));
          } else if (event.key === 'Enter' && isOpen && activeEntry) {
            event.preventDefault();
            selectEntry(activeEntry.entryKey);
          } else if (event.key === 'Enter' && isOpen && activeManualOption) {
            event.preventDefault();
            selectManual();
          } else if (event.key === 'Escape') {
            event.preventDefault();
            setIsOpen(false);
            setQuery('');
          }
        }}
      />
      {isOpen &&
        popoverPosition &&
        createPortal(
          <div
            ref={popoverRef}
            className="fixed z-[calc(var(--workspace-layer-modal)+1)]"
            style={popoverPosition}
          >
            <LiquidSurface
              liquidRole="overlay"
              className="w-full overflow-hidden rounded-box shadow-xl"
            >
              <ul
                className="menu flex w-full flex-col overflow-y-auto p-1"
                id={listId}
                role="listbox"
                style={{ maxHeight: popoverPosition.maxHeight }}
              >
                {visibleEntries.length === 0 && (
                  <li className="pointer-events-none px-3 py-2 text-sm opacity-60">{emptyLabel}</li>
                )}
                {visibleEntries.map((entry, index) => {
                  const suggestion = suggestions.find((item) => item.entryKey === entry.entryKey);
                  return (
                    <li key={entry.entryKey}>
                      <button
                        aria-selected={entry.entryKey === selectedEntryKey}
                        className={index === activeIndex ? 'active' : undefined}
                        id={`${listId}-${entry.entryKey}`}
                        role="option"
                        type="button"
                        onMouseDown={(event) => event.preventDefault()}
                        onMouseEnter={() => setActiveIndex(index)}
                        onClick={() => selectEntry(entry.entryKey)}
                      >
                        {entry.thumbnailPath ? (
                          <img
                            src={getFileUrl(entry.thumbnailPath)}
                            alt=""
                            className="size-10 shrink-0 rounded-md object-cover"
                          />
                        ) : (
                          <span className="grid size-10 shrink-0 place-items-center rounded-md bg-base-200 text-base-content/45">
                            <ImageIcon size={16} aria-hidden="true" />
                          </span>
                        )}
                        <span className="flex min-w-0 flex-1 flex-col items-start gap-1">
                          <span className="truncate font-medium">{entry.name}</span>
                          <span className="flex flex-wrap gap-1 text-xs opacity-80">
                            <span className="badge badge-ghost badge-xs">
                              {categoryLabel(entry.category)}
                            </span>
                            {suggestion && (
                              <span className="badge badge-primary badge-xs">
                                {suggestion.confidencePercentage}%
                              </span>
                            )}
                            {metadataChips(entry).map((chip) => (
                              <span className="badge badge-outline badge-xs" key={chip}>
                                {chip}
                              </span>
                            ))}
                          </span>
                        </span>
                      </button>
                    </li>
                  );
                })}
                {hasManualOption && (
                  <li className="mt-1 border-t border-base-300 pt-1">
                    <button
                      aria-selected={false}
                      className={activeManualOption ? 'active' : undefined}
                      id={`${listId}-manual`}
                      role="option"
                      type="button"
                      onMouseDown={(event) => event.preventDefault()}
                      onMouseEnter={() => setActiveIndex(visibleEntries.length)}
                      onClick={selectManual}
                    >
                      <span className="flex min-w-0 flex-1 flex-col items-start gap-0.5">
                        <span className="font-medium">{manualOptionLabel}</span>
                        {manualOptionHint && (
                          <span className="text-xs opacity-60">{manualOptionHint}</span>
                        )}
                      </span>
                    </button>
                  </li>
                )}
              </ul>
            </LiquidSurface>
          </div>,
          document.body,
        )}
    </div>
  );
}
