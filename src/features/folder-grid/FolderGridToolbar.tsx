import { Search, ChevronLeft, ArrowUpDown, LayoutGrid, List, RefreshCw } from 'lucide-react';
import ExplorerBreadcrumbs from './Breadcrumbs';

export interface FolderGridToolbarProps {
  isMobile: boolean;
  currentPath: string[];
  handleBreadcrumbClick: (index: number) => void;
  handleGoHome: () => void;
  selectedObject: string | null;
  setMobilePane: (pane: 'sidebar' | 'grid' | 'details') => void;
  handleSortToggle: () => void;
  sortLabel: string;
  sortOrder: 'asc' | 'desc';
  viewMode: 'grid' | 'list';
  setViewMode: (mode: 'grid' | 'list') => void;
  explorerSearchQuery: string;
  setExplorerSearch: (query: string) => void;
  visibleCount: number;
  handleRefresh: () => void;
}

export default function FolderGridToolbar({
  isMobile,
  currentPath,
  handleBreadcrumbClick,
  handleGoHome,
  selectedObject: _selectedObject,
  setMobilePane,
  handleSortToggle,
  sortLabel,
  sortOrder,
  viewMode,
  setViewMode,
  explorerSearchQuery,
  setExplorerSearch,
  visibleCount,
  handleRefresh,
}: FolderGridToolbarProps) {
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
            onGoHome={handleGoHome}
            isRootHidden
          />
        </div>

        {/* View/Sort toggle buttons */}
        <div className="flex items-center gap-1">
          <button
            onClick={handleSortToggle}
            className="btn btn-ghost btn-xs gap-1 text-base-content/50 hover:text-base-content"
            title={`Sort: ${sortLabel} ${sortOrder === 'asc' ? '↑' : '↓'}`}
          >
            <ArrowUpDown size={14} />
            <span className="text-[10px] font-semibold hidden sm:inline">
              {sortLabel} {sortOrder === 'asc' ? '↑' : '↓'}
            </span>
          </button>

          {!isMobile && (
            <>
              <button
                onClick={() => setViewMode('grid')}
                className={`btn btn-ghost btn-xs btn-square ${viewMode === 'grid' ? 'text-primary' : 'text-base-content/40'}`}
              >
                <LayoutGrid size={14} />
              </button>
              <button
                onClick={() => setViewMode('list')}
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
            placeholder="Search mods..."
            className="input input-sm w-full pl-10 bg-transparent border-transparent focus:border-transparent text-base-content placeholder:text-base-content/20 transition-all focus:bg-base-content/5 rounded-md"
            value={explorerSearchQuery}
            onChange={(e) => setExplorerSearch(e.target.value)}
          />
        </div>
        <span className="text-[10px] text-base-content/30 font-medium tabular-nums shrink-0">
          {visibleCount} item{visibleCount !== 1 ? 's' : ''}
        </span>
        <button
          onClick={handleRefresh}
          className="btn btn-ghost btn-xs btn-square text-base-content/30 hover:text-primary transition-colors"
          title="Refresh folder list"
        >
          <RefreshCw size={14} />
        </button>
      </div>
    </>
  );
}
