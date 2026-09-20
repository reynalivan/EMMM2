import {
  Search,
  ChevronLeft,
  ArrowUpDown,
  FolderPlus,
  LayoutGrid,
  List,
  LoaderCircle,
} from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { SortField, SortOrder } from '@/entities/mod';
import type { WorkspaceExplorerNode } from '@/entities/workspace';
import { LiquidSurface } from '@/shared/ui/liquid';
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
  hasMorePreviousFolders?: boolean;
  isLoadingMorePreviousFolders?: boolean;
  loadMorePreviousFolders?: () => Promise<unknown> | void;
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
  canCreateFolder: boolean;
  onCreateFolder: () => void;
  isRefreshing?: boolean;
}

export default function FolderGridToolbar({
  isMobile,
  currentPath,
  handleBreadcrumbClick,
  previousFolderItems,
  hasMorePreviousFolders,
  isLoadingMorePreviousFolders,
  loadMorePreviousFolders,
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
  canCreateFolder,
  onCreateFolder,
  isRefreshing = false,
}: FolderGridToolbarProps) {
  const { t } = useTranslation(['grid']);
  const [isSearchActive, setIsSearchActive] = useState(false);
  const [isSortOpen, setIsSortOpen] = useState(false);
  const sortMenuRef = useRef<HTMLDivElement>(null);
  const selectedSortOption =
    SORT_OPTIONS.find((option) => option.field === sortField && option.order === sortOrder) ??
    SORT_OPTIONS[0];
  const selectedSort = selectedSortOption.value;

  const handleSortChange = (value: string) => {
    const option = SORT_OPTIONS.find((candidate) => candidate.value === value);
    if (!option) return;

    setSortField(option.field);
    setSortOrder(option.order);
  };

  useEffect(() => {
    if (!isSortOpen) return;

    const closeWhenOutside = (event: MouseEvent) => {
      if (!sortMenuRef.current?.contains(event.target as Node)) {
        setIsSortOpen(false);
      }
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setIsSortOpen(false);
    };

    document.addEventListener('mousedown', closeWhenOutside);
    document.addEventListener('keydown', closeOnEscape);
    return () => {
      document.removeEventListener('mousedown', closeWhenOutside);
      document.removeEventListener('keydown', closeOnEscape);
    };
  }, [isSortOpen]);

  return (
    <LiquidSurface
      liquidRole="nav"
      className="folder-grid-action-bar relative z-30 mt-[var(--workspace-topbar-height)] block w-full shrink-0"
      contentClassName="h-auto"
      data-testid="folder-grid-toolbar"
    >
      <div
        className="folder-grid-toolbar flex min-h-14 flex-col gap-1 px-4 py-2"
        data-testid="folder-grid-toolbar-layout"
      >
        <div
          className="folder-grid-toolbar-breadcrumbs flex h-8 min-w-0 items-center gap-2"
          data-testid="folder-grid-toolbar-breadcrumbs"
        >
          <button
            onClick={() => setMobilePane('sidebar')}
            className="btn btn-ghost btn-sm btn-square text-base-content/50 hover:text-base-content md:hidden"
          >
            <ChevronLeft size={20} />
          </button>

          <ExplorerBreadcrumbs
            path={currentPath}
            onNavigate={handleBreadcrumbClick}
            previousFolderItems={previousFolderItems}
            hasMorePreviousFolders={hasMorePreviousFolders}
            isLoadingMorePreviousFolders={isLoadingMorePreviousFolders}
            loadMorePreviousFolders={loadMorePreviousFolders}
            onNavigateToPreviousFolder={handleNavigate}
            onGoHome={handleGoHome}
            isRootHidden
          />
          {isRefreshing && (
            <span className="flex shrink-0" role="status" aria-label={t('status.loading')}>
              <LoaderCircle
                size={14}
                className="animate-spin text-base-content/45 motion-reduce:animate-none"
                aria-hidden="true"
              />
            </span>
          )}
        </div>

        <div
          className="folder-grid-toolbar-controls flex w-full min-w-0 items-center justify-end gap-2"
          data-testid="folder-grid-toolbar-controls"
        >
          <label
            data-testid="mod-grid-search"
            className={`folder-grid-search group relative h-8 max-w-full shrink-0 transition-[width] duration-150 ease-out ${
              isSearchActive || explorerSearchQuery ? 'is-expanded w-56' : 'w-8'
            }`}
          >
            <span
              data-testid="mod-grid-search-icon"
              className="pointer-events-none absolute left-2.5 top-1/2 z-10 -translate-y-1/2 text-base-content/45 transition-colors group-focus-within:text-primary"
            >
              <Search size={16} aria-hidden="true" />
            </span>
            <input
              type="search"
              placeholder={t('toolbar.search_placeholder')}
              aria-label={t('toolbar.search_placeholder')}
              className={`input input-sm h-8 w-full border-base-content/8 bg-base-300/50 pl-9 pr-2 text-base-content placeholder:text-base-content/30 focus:bg-base-100 ${
                isSearchActive || explorerSearchQuery
                  ? 'opacity-100'
                  : 'cursor-pointer border-transparent bg-transparent opacity-0'
              }`}
              value={explorerSearchQuery}
              onChange={(event) => setExplorerSearch(event.target.value)}
              onFocus={() => setIsSearchActive(true)}
              onBlur={() => setIsSearchActive(false)}
            />
          </label>

          <button
            type="button"
            data-testid="add-folder"
            aria-label={t('toolbar.add_folder')}
            className="btn btn-ghost btn-sm btn-square shrink-0 text-base-content/80 hover:bg-base-content/6 hover:text-base-content"
            onClick={onCreateFolder}
            disabled={!canCreateFolder}
            title={t('toolbar.add_folder')}
          >
            <FolderPlus size={18} strokeWidth={2} aria-hidden="true" />
          </button>

          <div ref={sortMenuRef} className="relative shrink-0">
            <button
              type="button"
              id="folder-sort"
              data-testid="folder-sort"
              aria-label={t('toolbar.sort_label')}
              aria-expanded={isSortOpen}
              aria-haspopup="listbox"
              onClick={() => setIsSortOpen((open) => !open)}
              className="btn btn-ghost h-8 w-28 justify-between border border-base-content/12 bg-base-100/45 px-2 text-xs font-medium normal-case text-base-content hover:border-base-content/20 hover:bg-base-100/65"
            >
              <span className="min-w-0 truncate">{t(selectedSortOption.labelKey)}</span>
              <ArrowUpDown size={14} className="shrink-0 text-base-content/45" aria-hidden="true" />
            </button>

            {isSortOpen && (
              <LiquidSurface
                liquidRole="overlay"
                className="absolute right-0 top-full z-[var(--workspace-layer-popover)] mt-2 w-48 rounded-xl shadow-xl"
                contentClassName="p-1"
              >
                <div role="listbox" aria-label={t('toolbar.sort_label')}>
                  {SORT_OPTIONS.map((option) => {
                    const isSelected = option.value === selectedSort;
                    return (
                      <button
                        key={option.value}
                        type="button"
                        role="option"
                        aria-selected={isSelected}
                        className={`flex min-h-9 w-full items-center rounded-lg px-2.5 text-left text-sm transition-colors ${
                          isSelected
                            ? 'bg-base-content/10 text-base-content'
                            : 'text-base-content/75 hover:bg-base-content/8 hover:text-base-content'
                        }`}
                        onClick={() => {
                          handleSortChange(option.value);
                          setIsSortOpen(false);
                        }}
                      >
                        {t(option.labelKey)}
                      </button>
                    );
                  })}
                </div>
              </LiquidSurface>
            )}
          </div>

          {!isMobile && (
            <div className="folder-grid-view-controls flex shrink-0 items-center gap-1">
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
            </div>
          )}
        </div>
      </div>
    </LiquidSurface>
  );
}
