import { Ban, Search } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { ScanPreviewItem } from '../../../types/scanner';
import type { MasterDbEntry } from './scanReviewHelpers';

export type ScanReviewTab = 'All' | 'Matched' | 'Unmatched' | 'Existing' | 'Skipped';

interface ScanReviewFiltersProps {
  items: ScanPreviewItem[];
  overrides: Record<string, MasterDbEntry | null>;
  selectedCount: number;
  activeTab: ScanReviewTab;
  activeFilters: Set<string>;
  search: string;
  itemTab: (item: ScanPreviewItem) => string;
  onTabChange: (tab: ScanReviewTab) => void;
  onToggleFilter: (confidence: string) => void;
  onSearchChange: (search: string) => void;
  onDeclineSelected: () => void;
}

const TABS: ScanReviewTab[] = ['All', 'Matched', 'Unmatched', 'Existing', 'Skipped'];
const CONFIDENCES = ['Excellent', 'High', 'Medium', 'Low', 'Manual'];

export default function ScanReviewFilters(props: ScanReviewFiltersProps) {
  const { t } = useTranslation(['objects']);
  return (
    <>
      <div className="mt-2 mb-1 flex items-end justify-between">
        <div className="flex flex-1 gap-4 overflow-x-auto border-b border-base-content/10 px-1">
          {TABS.map((tab) => (
            <button
              key={tab}
              className={`relative flex shrink-0 items-center gap-1.5 px-2 pb-3 text-sm font-medium transition-colors focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-primary ${
                props.activeTab === tab
                  ? 'text-primary'
                  : 'text-base-content/60 hover:text-base-content/80'
              }`}
              onClick={() => props.onTabChange(tab)}
            >
              {t(`objects:scan_review.tabs.${tab.toLowerCase()}`, tab)}
              <span
                className={`text-[10px] uppercase font-bold px-1.5 py-0.5 leading-none rounded-full border ${tabPillClass(tab)}`}
              >
                {tab === 'All'
                  ? props.items.length
                  : props.items.filter((item) => props.itemTab(item) === tab).length}
              </span>
              {props.activeTab === tab && (
                <span className="absolute -bottom-px left-0 right-0 h-0.5 bg-primary" />
              )}
            </button>
          ))}
        </div>
        {props.selectedCount > 0 && (
          <button
            className="btn btn-xs h-7 min-h-0 btn-error btn-outline ml-4 mb-2.5"
            onClick={props.onDeclineSelected}
            title={t('objects:scan_review.bulk_skip_tip')}
          >
            <Ban size={12} /> {t('objects:scan_review.bulk_skip_label')} ({props.selectedCount})
          </button>
        )}
      </div>
      <div className="mt-3 mb-2 flex min-h-8 flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
        <div className="flex flex-wrap items-center gap-2">
          {props.activeTab !== 'Existing' &&
            CONFIDENCES.map((confidence) => (
              <button
                key={confidence}
                onClick={() => props.onToggleFilter(confidence)}
                className={`badge badge-sm h-6 cursor-pointer gap-1 pr-1 pl-2 transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary ${
                  props.activeFilters.has(confidence)
                    ? 'badge-primary'
                    : 'badge-outline border-base-300 text-base-content/60 hover:bg-base-200'
                }`}
              >
                {t(`objects:scan_review.tabs.${confidence.toLowerCase()}`, confidence)}
                <span className="text-[9px] rounded-full px-1 py-0.5 leading-none bg-base-300/50">
                  {confidenceCount(props, confidence)}
                </span>
              </button>
            ))}
        </div>
        <div className="relative w-full lg:ml-auto lg:w-64">
          <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 opacity-40" />
          <input
            type="text"
            className="input input-sm w-full pl-9"
            placeholder={t('objects:scan_review.search_placeholder')}
            value={props.search}
            onChange={(event) => props.onSearchChange(event.target.value)}
          />
        </div>
      </div>
    </>
  );
}

function confidenceCount(props: ScanReviewFiltersProps, confidence: string) {
  return props.items.filter((item) => {
    if (props.activeTab !== 'All' && props.itemTab(item) !== props.activeTab) return false;
    return (props.overrides[item.folderPath] ? 'Manual' : item.confidence) === confidence;
  }).length;
}

function tabPillClass(tab: ScanReviewTab) {
  if (tab === 'Matched') return 'bg-success/10 text-success border-success/20';
  if (tab === 'Unmatched') return 'bg-error/10 text-error border-error/20';
  if (tab === 'Skipped') return 'bg-warning/10 text-warning border-warning/20';
  return 'bg-base-300/50 text-base-content/60 border-transparent';
}
