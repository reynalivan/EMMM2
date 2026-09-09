import { Search, ChevronLeft, ArrowUpDown, LayoutGrid, List } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { SortField, SortOrder } from '@/entities/mod';
import type { WorkspaceExplorerNode } from '@/entities/workspace';
import ExplorerBreadcrumbs from './Breadcrumbs';

const SORT_OPTIONS = [
  { value: 'name:asc', field: 'name', order: 'asc', labelKey: 'toolbar.sort_name_asc' },
  { value: 'name:desc', field: 'name', order: 'desc', labelKey: 'toolbar.sort_name_desc' },
  {
    value: 'modified_at:asc',
    field: 'modified_at',
    order: 'asc',
    labelKey: 'toolbar.sort_date_asc',
  },
  {
    value: 'modified_at:desc',
    field: 'modified_at',
    order: 'desc',
    labelKey: 'toolbar.sort_date_desc',
  },
  { value: 'size_bytes:asc', field: 'size_bytes', order: 'asc', labelKey: 'toolbar.sort_size_asc' },
  {
    value: 'size_bytes:desc',
    field: 'size_bytes',
    order: 'desc',
    labelKey: 'toolbar.sort_size_desc',
  },
] as const;

export interface FolderGridToolbarProps {
  isMobile: boolean;
  currentPath: string[];
  handleBreadcrumbClick: (index: number) => void;
  previousFolderItems: WorkspaceExplorerNode[];
  handleNavigate: (folderName: string) => void;
  handleGoHome: () => void;
  setMobilePane: (pane: 'sidebar' | 'grid' | 'details') => void;
  sortField: SortField;
  sortOrder: SortOrder;
  setSortField: (field: SortField) => void;
  setSortOrder: (order: SortOrder) => void;
  viewMode: 'grid' | 'list';
  setViewMode: (mode: 'grid' | 'list') => void;
  explorerSearchQuery: string;
  setExplorerSearch: (query: string) => void;
  visibleCount: number;
}

export default function FolderGridToolbar({
  isMobile,
  currentPath,
  handleBreadcrumbClick,
  previousFolderItems,
  handleNavigate,
  handleGoHome,
  setMobilePane,
  sortField,
  sortOrder,
  setSortField,
  setSortOrder,
  viewMode,
  setViewMode,
  explorerSearchQuery,
  setExplorerSearch,
  visibleCount,
}: FolderGridToolbarProps) {
  const { t } = useTranslation(['grid']);
  const selectedSort =
    SORT_OPTIONS.find((option) => option.field === sortField && option.order === sortOrder)
      ?.value ?? SORT_OPTIONS[0].value;

  const handleSortChange = (value: string) => {
    const option = SORT_OPTIONS.find((candidate) => candidate.value === value);
    if (!option) return;

    setSortField(option.field);
    setSortOrder(option.order);
  };

  return (
    <>
      {/* Top Bar: Breadcrumbs & View Controls */}
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2 min-w-0 flex-1">
          <button
            onClick={() => setMobilePane('sidebar')}
            className="btn btn-ghost btn-sm btn-square md:hidden text-base-content/50 hover:text-base-content"
          >
            <ChevronLeft size={20} />
          </button>

          <ExplorerBreadcrumbs
            path={currentPath}
            onNavigate={handleBreadcrumbClick}
            previousFolderItems={previousFolderItems}
            onNavigateToPreviousFolder={handleNavigate}
            onGoHome={handleGoHome}
            isRootHidden
          />
        </div>

        {/* View and sort controls */}
        <div className="flex items-center gap-1">
          <div className="flex items-center gap-1.5 shrink-0">
            <ArrowUpDown size={14} className="text-base-content/50" aria-hidden="true" />
            <label htmlFor="folder-sort" className="text-[10px] font-semibold hidden sm:inline">
              {t('toolbar.sort_by')}
            </label>
            <select
              id="folder-sort"
              data-testid="folder-sort"
              aria-label={t('toolbar.sort_label')}
              value={selectedSort}
              onChange={(event) => handleSortChange(event.target.value)}
              className="select select-bordered select-xs w-32 sm:w-44 bg-base-100/60 text-base-content"
            >
              {SORT_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {t(option.labelKey)}
                </option>
              ))}
            </select>
          </div>

          {!isMobile && (
            <>
              <button
                data-testid="view-grid"
                onClick={() => setViewMode('grid')}
                aria-label={t('toolbar.view_grid')}
                aria-pressed={viewMode === 'grid'}
                title={t('toolbar.view_grid')}
                className={`btn btn-ghost btn-xs btn-square ${viewMode === 'grid' ? 'text-primary' : 'text-base-content/40'}`}
              >
                <LayoutGrid size={14} />
              </button>
              <button
                data-testid="view-list"
                onClick={() => setViewMode('list')}
                aria-label={t('toolbar.view_list')}
                aria-pressed={viewMode === 'list'}
                title={t('toolbar.view_list')}
                className={`btn btn-ghost btn-xs btn-square ${viewMode === 'list' ? 'text-primary' : 'text-base-content/40'}`}
              >
                <List size={14} />
              </button>
            </>
          )}
        </div>
      </div>

      {/* Search toolbar */}
      <div className="flex items-center gap-3 mb-3 bg-base-300/50 p-2 rounded-lg border border-base-content/5">
        <div className="relative flex-1 group">
          <Search
            className="absolute left-3 top-1/2 -translate-y-1/2 text-base-content/30 group-focus-within:text-primary transition-colors"
            size={16}
          />
          <input
            type="text"
            placeholder={t('toolbar.search_placeholder')}
            className="input input-sm w-full pl-10 bg-transparent border-transparent focus:border-transparent text-base-content placeholder:text-base-content/20 transition-all focus:bg-base-content/5 rounded-md"
            value={explorerSearchQuery}
            onChange={(e) => setExplorerSearch(e.target.value)}
          />
        </div>
        <span className="text-[10px] text-base-content/30 font-medium tabular-nums shrink-0">
          {t('toolbar.item_count', { count: visibleCount })}
        </span>
      </div>
    </>
  );
}
