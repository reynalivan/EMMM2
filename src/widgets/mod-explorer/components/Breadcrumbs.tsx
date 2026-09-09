/**
 * Epic 4: Breadcrumbs — Path navigation for the mod explorer.
 * Shows clickable path segments with overflow truncation.
 */

import { Folder, Home, Search, Star } from 'lucide-react';
import { useRef, useState, type FocusEvent, type KeyboardEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { useAppStore } from '@/app/store';
import { useThumbnail } from '@/entities/mod';
import type { WorkspaceExplorerNode } from '@/entities/workspace';

interface BreadcrumbsProps {
  path: string[];
  onNavigate: (index: number) => void;
  previousFolderItems?: WorkspaceExplorerNode[];
  onNavigateToPreviousFolder?: (name: string) => void;
  onGoHome: () => void;
  isRootHidden?: boolean;
}

interface PreviousFolderItemProps {
  folder: WorkspaceExplorerNode;
  onNavigate: (name: string) => void;
}

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
            className="size-full object-cover transition-transform duration-200 group-hover:scale-105"
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
  onNavigateToPreviousFolder,
  onGoHome,
  isRootHidden = false,
}: BreadcrumbsProps) {
  const { t } = useTranslation('folder_grid');
  const [isPreviousFolderOpen, setIsPreviousFolderOpen] = useState(false);
  const [previousFolderSearch, setPreviousFolderSearch] = useState('');
  const previousFolderButtonRef = useRef<HTMLButtonElement>(null);
  // Truncate middle segments when path is too deep
  const MAX_VISIBLE = 4;
  const shouldTruncate = path.length > MAX_VISIBLE;

  const visiblePath = shouldTruncate ? [path[0], '...', ...path.slice(-2)] : path;

  // Map visible indices back to real path indices for navigation
  const getRealIndex = (visibleIndex: number): number => {
    if (!shouldTruncate) return visibleIndex;
    if (visibleIndex === 0) return 0;
    if (visibleIndex === 1) return -1; // "..." placeholder — not clickable
    return path.length - visiblePath.length + visibleIndex;
  };
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

  return (
    <div className="breadcrumbs text-sm text-base-content/50 font-medium min-w-0 overflow-visible">
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
        {visiblePath.map((segment, i) => {
          const realIndex = getRealIndex(i);
          const isPlaceholder = segment === '...';
          const isPreviousFolder = realIndex === previousPathIndex && onNavigateToPreviousFolder;

          return (
            <li key={`${segment}-${i}`}>
              {isPlaceholder ? (
                <span className="text-base-content/30 text-xs">…</span>
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
                    title={segment}
                  >
                    {segment}
                  </button>

                  {isPreviousFolderOpen && (
                    <div className="absolute left-0 top-full z-50 pt-2">
                      <div
                        id="breadcrumb-previous-folder-menu"
                        role="dialog"
                        aria-label={t('breadcrumbs.previous_folder_menu', { folder: segment })}
                        className="w-[40rem] max-w-[calc(100vw-2rem)] animate-in fade-in-0 zoom-in-95 rounded-xl border border-base-content/10 bg-base-100/95 p-3 shadow-2xl backdrop-blur-xl duration-150"
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

                        <div className="custom-scrollbar mt-3 grid max-h-72 grid-cols-2 gap-1.5 overflow-y-auto pr-1">
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
                    </div>
                  )}
                </div>
              ) : (
                <button
                  onClick={() => onNavigate(realIndex)}
                  className="hover:text-primary transition-colors hover:underline truncate max-w-30 text-xs"
                  title={segment}
                >
                  {segment}
                </button>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
