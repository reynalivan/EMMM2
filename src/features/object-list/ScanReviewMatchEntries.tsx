import { Search } from 'lucide-react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import { type MasterDbEntry, getConfidenceColor } from './scanReviewHelpers';

export type MergedScanReviewEntry = MasterDbEntry & { scorePct?: number };

export function SearchInput({
  searchQuery,
  onSearchChange,
}: {
  searchQuery: string;
  onSearchChange: (value: string) => void;
}) {
  const { t } = useTranslation(['objects']);

  return (
    <div className="p-2 border-b border-base-300/30 bg-base-300/30">
      <div className="relative">
        <Search
          size={14}
          className="absolute left-2.5 top-1/2 -translate-y-1/2 text-base-content/40"
        />
        <input
          type="text"
          className="input input-sm w-full pl-8 bg-base-100/60 border-base-300/30 placeholder:text-base-content/30"
          placeholder={t('context.sync.search_placeholder')}
          value={searchQuery}
          onChange={(event) => onSearchChange(event.target.value)}
          autoFocus
        />
      </div>
    </div>
  );
}

export function EntryGroup({
  entries,
  label,
  totalCount,
  onSelect,
  variant,
}: {
  entries: MergedScanReviewEntry[];
  label: string;
  totalCount: number;
  onSelect: (entry: MasterDbEntry) => void;
  variant: 'candidate' | 'other';
}) {
  if (totalCount === 0) {
    return null;
  }

  return (
    <>
      <div className="px-3 py-1.5 text-[10px] uppercase tracking-widest text-base-content/40 font-semibold bg-base-300/20 border-b border-base-300/20 sticky top-0 z-10 backdrop-blur-sm">
        {label} ({totalCount})
      </div>
      {entries.map((entry) =>
        variant === 'candidate' ? (
          <CandidateEntryButton key={entry.name} entry={entry} onSelect={onSelect} />
        ) : (
          <OtherEntryButton key={entry.name} entry={entry} onSelect={onSelect} />
        ),
      )}
    </>
  );
}

function CandidateEntryButton({
  entry,
  onSelect,
}: {
  entry: MergedScanReviewEntry;
  onSelect: (entry: MasterDbEntry) => void;
}) {
  return (
    <button
      className="flex flex-col gap-0.5 px-3 py-2 hover:bg-base-300/30 transition-colors text-left w-full border-b border-base-300/10 last:border-b-0"
      onClick={() => onSelect(entry)}
    >
      <div className="flex items-center gap-2 w-full">
        {entry.thumbnail_path ? (
          <div className="avatar">
            <div className="w-6 rounded-full bg-base-300 ring-1 ring-base-300/50">
              <img src={convertFileSrc(entry.thumbnail_path)} alt="" />
            </div>
          </div>
        ) : (
          <div className="w-6 h-6 rounded-full bg-base-300/50 flex items-center justify-center">
            <Search size={10} className="opacity-30" />
          </div>
        )}
        <span className="truncate font-semibold text-sm flex-1">{entry.name}</span>
        <span className={`badge badge-xs font-mono tabular-nums ${getScoreColor(entry)}`}>
          {entry.scorePct}%
        </span>
      </div>
      <div className="flex items-center gap-1 ml-8 flex-wrap">
        <span className="badge badge-xs bg-base-300/50 border-base-300/60 text-base-content/60 uppercase text-[9px]">
          {entry.object_type}
        </span>
        {entry.tags.slice(0, 3).map((tag) => (
          <span key={tag} className="badge badge-xs badge-ghost text-[9px] text-base-content/40">
            {tag}
          </span>
        ))}
      </div>
    </button>
  );
}

function OtherEntryButton({
  entry,
  onSelect,
}: {
  entry: MergedScanReviewEntry;
  onSelect: (entry: MasterDbEntry) => void;
}) {
  return (
    <button
      className="flex items-center gap-2 px-3 py-1.5 hover:bg-base-300/30 transition-colors text-left w-full"
      onClick={() => onSelect(entry)}
    >
      {entry.thumbnail_path ? (
        <div className="avatar">
          <div className="w-5 rounded-full bg-base-300">
            <img src={convertFileSrc(entry.thumbnail_path)} alt="" />
          </div>
        </div>
      ) : (
        <div className="w-5 h-5 rounded-full bg-base-300/40" />
      )}
      <span className="truncate font-medium text-sm flex-1 text-base-content/70">{entry.name}</span>
      {entry.scorePct !== undefined ? (
        <span className="badge badge-xs bg-transparent border-0 font-mono text-[10px] text-base-content/40 tabular-nums">
          {entry.scorePct}%
        </span>
      ) : (
        <div className="w-6 h-3 rounded bg-base-300/40 animate-pulse ml-2"></div>
      )}
      <span className="badge badge-xs bg-base-300/40 border-base-300/50 text-base-content/40 uppercase text-[9px]">
        {entry.object_type}
      </span>
    </button>
  );
}

function getScoreColor(entry: MergedScanReviewEntry): string {
  const score = entry.scorePct ?? 0;
  if (score >= 90) {
    return getConfidenceColor('Excellent');
  }

  if (score >= 75) {
    return getConfidenceColor('High');
  }

  if (score >= 45) {
    return getConfidenceColor('Medium');
  }

  return getConfidenceColor('Low');
}
