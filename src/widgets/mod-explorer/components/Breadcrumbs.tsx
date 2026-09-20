/**
 * Epic 4: Breadcrumbs — Path navigation for the mod explorer.
 * Shows clickable path segments with overflow truncation.
 */

import { Folder, Home, Search, Star } from 'lucide-react';
import {
  useEffect,
  useRef,
  useState,
  type FocusEvent,
  type KeyboardEvent,
  type UIEvent,
} from 'react';
import { useTranslation } from 'react-i18next';
import { useAppStore } from '@/app/store';
import { useThumbnail } from '@/entities/mod';
import type { WorkspaceExplorerNode } from '@/entities/workspace';
import { LiquidSurface } from '@/shared/ui/liquid';

interface BreadcrumbsProps {
  path: string[];
  onNavigate: (index: number) => void;
  previousFolderItems?: WorkspaceExplorerNode[];
  hasMorePreviousFolders?: boolean;
  isLoadingMorePreviousFolders?: boolean;
  loadMorePreviousFolders?: () => Promise<unknown> | void;
  onNavigateToPreviousFolder?: (name: string) => void;
  onGoHome: () => void;
  isRootHidden?: boolean;
}

interface PreviousFolderItemProps {
  folder: WorkspaceExplorerNode;
  onNavigate: (name: string) => void;
}

interface VisiblePathSegment {
  label: string;
  realIndex: number | null;
}

const COMPACT_BREADCRUMB_WIDTH = 260;

function PreviousFolderItem({ folder, onNavigate }: PreviousFolderItemProps) {
  const activeGameId = useAppStore((state) => state.activeGameId);
  const { data: thumbnailSrc } = useThumbnail(activeGameId ?? '', folder.path);

  return (
    <button
      type="button"
      onClick={() => onNavigate(folder.name)}
      className="group flex min-w-0 items-center gap-3 rounded-lg border border-transparent px-2 py-2 text-left transition-[background-color,border-color] duration-150 hover:border-base-content/8 hover:bg-base-content/6 focus-visible:border-primary/50 focus-visible:bg-primary/8 focus-visible:outline-none"
    >
      <span className="grid size-10 shrink-0 overflow-hidden rounded-lg bg-base-300/70 ring-1 ring-base-content/6">
        {thumbnailSrc ? (
          <img
            src={thumbnailSrc}
            alt=""
            className="size-full object-cover transition-opacity duration-150 group-hover:opacity-85"
          />
        ) : (
          <Folder size={16} className="m-auto text-base-content/35" aria-hidden="true" />
        )}
      </span>
      <span className="min-w-0 flex-1 truncate text-sm font-medium text-base-content/80 group-hover:text-base-content">
        {folder.name}
      </span>
      {folder.is_favorite && (
        <Star size={14} className="shrink-0 fill-warning text-warning" aria-hidden="true" />
      )}
    </button>
  );
}

export default function ExplorerBreadcrumbs({
  path,
  onNavigate,
  previousFolderItems = [],
  hasMorePreviousFolders = false,
  isLoadingMorePreviousFolders = false,
  loadMorePreviousFolders,
  onNavigateToPreviousFolder,
  onGoHome,
  isRootHidden = false,
}: BreadcrumbsProps) {
  const { t } = useTranslation('folder_grid');
  const [isPreviousFolderOpen, setIsPreviousFolderOpen] = useState(false);
  const [previousFolderSearch, setPreviousFolderSearch] = useState('');
  const [isCompact, setIsCompact] = useState(false);
  const breadcrumbRef = useRef<HTMLDivElement>(null);
  const previousFolderButtonRef = useRef<HTMLButtonElement>(null);
  const maxVisibleSegments = isCompact ? 2 : 4;
  const MAX_VISIBLE = maxVisibleSegments;
  const shouldTruncate = path.length > MAX_VISIBLE;
  const visiblePath: VisiblePathSegment[] = shouldTruncate
    ? isCompact
      ? [
          { label: '…', realIndex: null },
          { label: path[path.length - 1], realIndex: path.length - 1 },
        ]
      : [
          { label: path[0], realIndex: 0 },
          { label: '…', realIndex: null },
          ...path.slice(-2).map((label, index) => ({
            label,
            realIndex: path.length - 2 + index,
          })),
        ]
    : path.map((label, realIndex) => ({ label, realIndex }));
  const previousPathIndex = path.length - 2;
  const previousFolders = [...previousFolderItems]
    .filter((folder) =>
      folder.name.toLocaleLowerCase().includes(previousFolderSearch.toLocaleLowerCase()),
    )
    .sort((left, right) => {
      if (left.is_favorite !== right.is_favorite) {
        return left.is_favorite ? -1 : 1;
      }

      return left.name.localeCompare(right.name, undefined, { sensitivity: 'base' });
    });

  const closePreviousFolder = () => setIsPreviousFolderOpen(false);

  useEffect(() => {
    const element = breadcrumbRef.current;
    if (!element || !('ResizeObserver' in window)) return;

    const observer = new ResizeObserver(([entry]) => {
      if (!entry) return;
      setIsCompact(entry.contentRect.width < COMPACT_BREADCRUMB_WIDTH);
    });

    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const handlePreviousFolderBlur = (event: FocusEvent<HTMLDivElement>) => {
    if (!event.currentTarget.contains(event.relatedTarget)) {
      closePreviousFolder();
    }
  };
  const handlePreviousFolderKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== 'Escape') return;

    event.preventDefault();
    closePreviousFolder();
    previousFolderButtonRef.current?.focus();
  };
  const handlePreviousFolderScroll = (event: UIEvent<HTMLDivElement>) => {
    if (!hasMorePreviousFolders || isLoadingMorePreviousFolders || !loadMorePreviousFolders) {
      return;
    }
    const list = event.currentTarget;
    if (list.scrollHeight - list.scrollTop - list.clientHeight <= 64) {
      void loadMorePreviousFolders();
    }
  };

  return (
    <div
      ref={breadcrumbRef}
      className="breadcrumbs text-sm text-base-content/50 font-medium min-w-0 overflow-visible"
    >
      <ul className="flex-nowrap">
        {!isRootHidden && (
          <li>
            <button
              onClick={onGoHome}
              className="hover:text-primary transition-colors flex items-center gap-1"
            >
              <Home size={14} />
              <span className="hidden sm:inline text-xs">{t('breadcrumbs.root')}</span>
            </button>
          </li>
        )}
        {visiblePath.map(({ label, realIndex }, index) => {
          const isPlaceholder = realIndex === null;
          const isPreviousFolder = realIndex === previousPathIndex && onNavigateToPreviousFolder;

          return (
            <li key={`${label}-${index}`}>
              {isPlaceholder ? (
                <span
                  className="text-base-content/30 text-xs"
                  title={path.slice(0, -1).join(' / ')}
                >
                  …
                </span>
              ) : isPreviousFolder ? (
                <div
                  className="relative"
                  onMouseEnter={() => setIsPreviousFolderOpen(true)}
                  onMouseLeave={closePreviousFolder}
                  onFocus={() => setIsPreviousFolderOpen(true)}
                  onBlur={handlePreviousFolderBlur}
                  onKeyDown={handlePreviousFolderKeyDown}
                >
                  <button
                    ref={previousFolderButtonRef}
                    type="button"
                    onClick={() => onNavigate(realIndex)}
                    aria-expanded={isPreviousFolderOpen}
                    aria-controls="breadcrumb-previous-folder-menu"
                    className="hover:text-primary transition-colors hover:underline truncate max-w-30 text-xs"
                    title={label}
                  >
                    {label}
                  </button>

                  {isPreviousFolderOpen && (
                    <div className="absolute left-0 top-full z-50 pt-2">
                      <LiquidSurface
                        liquidRole="overlay"
                        className="w-[40rem] max-w-[calc(100vw-2rem)] animate-in fade-in-0 zoom-in-95 rounded-xl shadow-xl duration-150"
                      >
                        <div
                          id="breadcrumb-previous-folder-menu"
                          role="dialog"
                          aria-label={t('breadcrumbs.previous_folder_menu', { folder: label })}
                          className="p-3"
                        >
                          <label className="input input-sm flex h-10 items-center gap-2 rounded-lg border-base-content/8 bg-base-200/65 px-3 shadow-none">
                            <Search size={15} className="text-base-content/45" aria-hidden="true" />
                            <input
                              autoFocus
                              type="search"
                              value={previousFolderSearch}
                              onChange={(event) => setPreviousFolderSearch(event.target.value)}
                              placeholder={t('breadcrumbs.search_previous_folder')}
                              aria-label={t('breadcrumbs.search_previous_folder')}
                              className="grow text-sm"
                            />
                          </label>

                          <div
                            data-testid="breadcrumb-previous-folder-list"
                            className="custom-scrollbar mt-3 grid max-h-72 grid-cols-2 gap-1.5 overflow-y-auto pr-1"
                            onScroll={handlePreviousFolderScroll}
                          >
                            {previousFolders.map((folder) => (
                              <PreviousFolderItem
                                key={folder.path}
                                folder={folder}
                                onNavigate={onNavigateToPreviousFolder}
                              />
                            ))}
                          </div>

                          {previousFolders.length === 0 && (
                            <p className="px-1 py-3 text-center text-xs text-base-content/50">
                              {t('breadcrumbs.no_previous_folder_results')}
                            </p>
                          )}
                        </div>
                      </LiquidSurface>
                    </div>
                  )}
                </div>
              ) : (
                <button
                  onClick={() => onNavigate(realIndex)}
                  className="hover:text-primary transition-colors hover:underline truncate max-w-30 text-xs"
                  title={label}
                >
                  {label}
                </button>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
