/**
 * Epic 4: Breadcrumbs — Path navigation for the mod explorer.
 * Shows clickable path segments with overflow truncation.
 */

import { Home } from 'lucide-react';

interface BreadcrumbsProps {
  path: string[];
  onNavigate: (index: number) => void;
  onGoHome: () => void;
  isRootHidden?: boolean;
}

export default function ExplorerBreadcrumbs({
  path,
  onNavigate,
  onGoHome,
  isRootHidden = false,
}: BreadcrumbsProps) {
  // Truncate middle segments when path is too deep
  const MAX_VISIBLE = 4;
  const shouldTruncate = path.length > MAX_VISIBLE;

  const visiblePath = shouldTruncate ? [path[0], '...', ...path.slice(-2)] : path;

  // Map visible indices back to real path indices for navigation
  const getRealIndex = (visibleIndex: number): number => {
    if (!shouldTruncate) return visibleIndex;
    if (visibleIndex === 0) return 0;
    if (visibleIndex === 1) return -1; // "..." placeholder — not clickable
    return path.length - (visiblePath.length - 1 - visibleIndex);
  };

  return (
    <div className="breadcrumbs text-sm text-base-content/50 font-medium min-w-0 overflow-hidden">
      <ul className="flex-nowrap">
        {!isRootHidden && (
          <li>
            <button
              onClick={onGoHome}
              className="hover:text-primary transition-colors flex items-center gap-1"
            >
              <Home size={14} />
              <span className="hidden sm:inline text-xs">ROOT</span>
            </button>
          </li>
        )}
        {visiblePath.map((segment, i) => {
          const realIndex = getRealIndex(i);
          const isPlaceholder = segment === '...';

          return (
            <li key={`${segment}-${i}`}>
              {isPlaceholder ? (
                <span className="text-base-content/30 text-xs">…</span>
              ) : (
                <button
                  onClick={() => onNavigate(realIndex)}
                  className="hover:text-primary transition-colors hover:underline truncate max-w-[120px] text-xs"
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
