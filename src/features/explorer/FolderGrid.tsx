/* eslint-disable react-hooks/set-state-in-effect */
import { useState, useEffect, useRef } from 'react';
import { Home, Search, Filter, Plus, ChevronLeft, ChevronUp, ChevronDown } from 'lucide-react';
import { useAppStore } from '../../stores/useAppStore';
import { useResponsive } from '../../hooks/useResponsive';
import { generateDummyItems } from '../../lib/mockData';
import FolderCard from './FolderCard';
import FolderListRow from './FolderListRow';

export default function FolderGrid() {
  const {
    currentPath,
    setCurrentPath,
    gridSelection,
    toggleGridSelection,
    clearGridSelection,
    setMobilePane,
  } = useAppStore();

  const [items] = useState(() => generateDummyItems(64));
  const [searchQuery, setSearchQuery] = useState('');
  const { isMobile } = useResponsive();

  // Scroll Indicator State
  const containerRef = useRef<HTMLDivElement>(null);
  const [indicator, setIndicator] = useState<'up' | 'down' | null>(null);

  // Scroll Tracking for Indicators
  useEffect(() => {
    const container = containerRef.current;
    if (!container || gridSelection.size === 0) {
      setIndicator(null);
      return;
    }

    const checkVisibility = () => {
      const firstSelectedId = Array.from(gridSelection)[0];
      if (!firstSelectedId) return;

      const element = document.getElementById(`grid-item-${firstSelectedId}`);
      if (!element) return;

      const containerRect = container.getBoundingClientRect();
      const itemRect = element.getBoundingClientRect();

      const isAbove = itemRect.bottom < containerRect.top + 50;
      const isBelow = itemRect.top > containerRect.bottom - 50;

      if (isAbove) setIndicator('up');
      else if (isBelow) setIndicator('down');
      else setIndicator(null);
    };

    container.addEventListener('scroll', checkVisibility);
    checkVisibility();

    return () => container.removeEventListener('scroll', checkVisibility);
  }, [gridSelection, items]);

  const scrollToSelection = () => {
    const firstSelectedId = Array.from(gridSelection)[0];
    if (!firstSelectedId) return;
    const element = document.getElementById(`grid-item-${firstSelectedId}`);
    element?.scrollIntoView({ behavior: 'smooth', block: 'center' });
  };

  const handleBreadcrumbClick = (index: number) => {
    setCurrentPath(currentPath.slice(0, index + 1));
  };

  const handleNavigate = (folderName: string) => {
    setCurrentPath([...currentPath, folderName]);
  };

  return (
    <div className="flex flex-col h-full bg-transparent p-4 relative">
      {/* Top Bar: Breadcrumbs & Actions */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <button
            onClick={() => setMobilePane('sidebar')}
            className="btn btn-ghost btn-sm btn-square md:hidden text-base-content/50 hover:text-base-content"
          >
            <ChevronLeft size={20} />
          </button>

          <div className="breadcrumbs text-sm text-base-content/50 font-medium">
            <ul>
              <li>
                <button
                  onClick={() => setCurrentPath([])}
                  className="hover:text-primary transition-colors flex items-center gap-1"
                >
                  <Home size={16} /> <span className="hidden sm:inline">ROOT</span>
                </button>
              </li>
              {currentPath.map((folder, index) => (
                <li key={folder}>
                  <button
                    onClick={() => handleBreadcrumbClick(index)}
                    className="hover:text-primary transition-colors hover:underline"
                  >
                    {folder}
                  </button>
                </li>
              ))}
            </ul>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <button className="btn btn-primary btn-sm gap-2 shadow-sm shadow-primary/20 text-white font-medium">
            <Plus size={16} />
            <span className="hidden sm:inline">Create</span>
          </button>
        </div>
      </div>

      {/* Toolbar */}
      <div className="flex items-center gap-3 mb-4 bg-base-300/50 p-2 rounded-lg border border-base-content/5">
        <div className="relative flex-1 group">
          <Search
            className="absolute left-3 top-1/2 -translate-y-1/2 text-base-content/30 group-focus-within:text-primary transition-colors"
            size={16}
          />
          <input
            type="text"
            placeholder="Search folder..."
            className="input input-sm w-full pl-10 bg-transparent border-transparent focus:border-transparent text-base-content placeholder:text-base-content/20 transition-all focus:bg-base-content/5 rounded-md"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
        </div>
        <div className="h-6 w-px bg-base-content/10 mx-1" />
        <button className="btn btn-sm btn-ghost btn-square text-base-content/50 hover:text-base-content hover:bg-base-content/5">
          <Filter size={18} />
        </button>
        <label className="label cursor-pointer gap-2 py-0 hover:opacity-100 opacity-60 transition-opacity">
          <span className="label-text text-xs text-base-content/70 font-medium">ALL</span>
          <input
            type="checkbox"
            className="checkbox checkbox-xs border-base-content/30 checked:border-primary checkbox-primary rounded-[4px]"
          />
        </label>
      </div>

      {/* Grid Content */}
      <div
        ref={containerRef}
        className={`overflow-y-auto pb-24 scroll-p-4 scrollbar-thin scrollbar-track-transparent scrollbar-thumb-base-content/20 hover:scrollbar-thumb-base-content/40 ${
          isMobile
            ? 'flex flex-col gap-3'
            : 'grid gap-4 grid-cols-[repeat(auto-fill,minmax(220px,1fr))]'
        }`}
      >
        {items.map((item) => {
          const isSelected = gridSelection.has(item.id);

          if (isMobile) {
            return (
              <FolderListRow
                key={item.id}
                item={item}
                isSelected={isSelected}
                toggleSelection={toggleGridSelection}
                clearSelection={clearGridSelection}
              />
            );
          }

          return (
            <FolderCard
              key={item.id}
              item={item}
              isSelected={isSelected}
              onNavigate={handleNavigate}
              toggleSelection={toggleGridSelection}
              clearSelection={clearGridSelection}
            />
          );
        })}
      </div>

      {/* Floating Scroll Indicators */}
      {indicator && (
        <div
          className={`absolute left-1/2 -translate-x-1/2 z-20 flex flex-col items-center gap-1 pointer-events-auto cursor-pointer ${indicator === 'up' ? 'top-32' : 'bottom-6'}`}
          onClick={scrollToSelection}
        >
          <div className="px-3 py-1.5 bg-primary text-white text-xs font-bold rounded-full shadow-lg flex items-center gap-1.5 hover:bg-primary/90 transition-colors">
            {indicator === 'up' ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
            <span>Selection {indicator === 'up' ? 'Above' : 'Below'}</span>
          </div>
        </div>
      )}
    </div>
  );
}
