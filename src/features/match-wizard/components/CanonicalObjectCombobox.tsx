import { useId, useMemo, useState } from 'react';
import type {
  CanonicalClassificationCatalogEntry,
  CanonicalSuggestion,
} from '../../../shared/api/tauri/bindings.gen';

type CanonicalObjectComboboxProps = {
  disabled?: boolean;
  emptyLabel: string;
  entries: CanonicalClassificationCatalogEntry[];
  suggestions: CanonicalSuggestion[];
  selectedEntryKey: string | null;
  onSelect: (entryKey: string) => void;
};

const MAX_VISIBLE_RESULTS = 24;

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

export function CanonicalObjectCombobox({
  disabled = false,
  emptyLabel,
  entries,
  suggestions,
  selectedEntryKey,
  onSelect,
}: CanonicalObjectComboboxProps) {
  const listId = useId();
  const [isOpen, setIsOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [activeIndex, setActiveIndex] = useState(0);
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

  const selectEntry = (entryKey: string) => {
    onSelect(entryKey);
    setQuery('');
    setActiveIndex(0);
    setIsOpen(false);
  };

  return (
    <div className="relative min-w-72">
      <input
        aria-activedescendant={
          isOpen && activeEntry ? `${listId}-${activeEntry.entryKey}` : undefined
        }
        aria-autocomplete="list"
        aria-controls={listId}
        aria-expanded={isOpen}
        aria-haspopup="listbox"
        aria-label="Canonical object"
        className="input input-bordered input-sm w-full"
        disabled={disabled}
        placeholder={selected?.name ?? 'Search canonical object'}
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
            setActiveIndex((current) =>
              Math.min(current + 1, Math.max(visibleEntries.length - 1, 0)),
            );
          } else if (event.key === 'ArrowUp') {
            event.preventDefault();
            setIsOpen(true);
            setActiveIndex((current) => Math.max(current - 1, 0));
          } else if (event.key === 'Enter' && isOpen && activeEntry) {
            event.preventDefault();
            selectEntry(activeEntry.entryKey);
          } else if (event.key === 'Escape') {
            event.preventDefault();
            setIsOpen(false);
            setQuery('');
          }
        }}
      />
      {isOpen && (
        <ul
          className="menu absolute z-30 mt-1 max-h-72 w-full overflow-y-auto rounded-box border border-base-300 bg-base-100 p-1 shadow-xl"
          id={listId}
          role="listbox"
        >
          {visibleEntries.length === 0 ? (
            <li className="pointer-events-none px-3 py-2 text-sm opacity-60">{emptyLabel}</li>
          ) : (
            visibleEntries.map((entry, index) => {
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
                    <span className="flex min-w-0 flex-col items-start">
                      <span className="truncate font-medium">{entry.name}</span>
                      <span className="text-xs opacity-70">
                        {entry.category}
                        {suggestion ? ` · ${suggestion.confidencePercentage}%` : ''}
                      </span>
                    </span>
                  </button>
                </li>
              );
            })
          )}
        </ul>
      )}
    </div>
  );
}
