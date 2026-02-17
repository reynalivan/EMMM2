/**
 * useObjectListVirtualizer — virtualizer, sticky header, and Object Mode
 * data-shaping logic extracted from useObjectListLogic to keep it under 350 lines.
 */

import { useState, useMemo, useCallback, useEffect, useRef } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import type { ObjectSummary, GameSchema, CategoryDef } from '../../types/object';

/** Discriminated union for flat list items in Object Mode */
export type FlatItem =
  | { type: 'header'; category: CategoryDef; count: number }
  | { type: 'sub-header'; label: string; parentCategory: string; count: number }
  | { type: 'row'; obj: ObjectSummary };

interface VirtualizerOptions {
  objects: ObjectSummary[];
  schema: GameSchema | undefined;
  selectedObject: string | null;
  isMobile: boolean;
}

export function useObjectListVirtualizer({
  objects,
  schema,
  selectedObject,
  isMobile,
}: VirtualizerOptions) {
  const parentRef = useRef<HTMLDivElement>(null);

  // Group objects by category
  const groupedObjects = useMemo(() => {
    const groups: Record<string, ObjectSummary[]> = {};
    for (const obj of objects) {
      const key = obj.object_type || 'Other';
      if (!groups[key]) groups[key] = [];
      groups[key].push(obj);
    }
    return groups;
  }, [objects]);

  // Flatten: no collapse — always show all items. Skip empty categories.
  const flatObjectItems = useMemo((): FlatItem[] => {
    const categories = schema?.categories ?? [
      { name: 'Character', icon: 'User', color: 'primary' },
      { name: 'Weapon', icon: 'Sword', color: 'secondary' },
      { name: 'UI', icon: 'Layout', color: 'accent' },
      { name: 'Other', icon: 'Package', color: 'neutral' },
    ];

    const items: FlatItem[] = [];
    const matchedTypes = new Set<string>();

    for (const cat of categories) {
      const catObjects = groupedObjects[cat.name] ?? [];
      const count = catObjects.length;
      matchedTypes.add(cat.name);

      // Skip empty categories entirely
      if (count === 0) continue;

      items.push({ type: 'header', category: cat, count });

      // "Other" category: sub-group by sub_category
      if (cat.name === 'Other' && catObjects.length > 0) {
        const subGroups: Record<string, ObjectSummary[]> = {};
        const noSubCat: ObjectSummary[] = [];
        for (const obj of catObjects) {
          if (obj.sub_category) {
            if (!subGroups[obj.sub_category]) subGroups[obj.sub_category] = [];
            subGroups[obj.sub_category].push(obj);
          } else {
            noSubCat.push(obj);
          }
        }

        for (const subCat of Object.keys(subGroups).sort()) {
          const subObjects = subGroups[subCat];
          items.push({
            type: 'sub-header',
            label: subCat,
            parentCategory: cat.name,
            count: subObjects.length,
          });
          for (const obj of subObjects) {
            items.push({ type: 'row', obj });
          }
        }

        for (const obj of noSubCat) {
          items.push({ type: 'row', obj });
        }
      } else {
        for (const obj of catObjects) {
          items.push({ type: 'row', obj });
        }
      }
    }

    // Collect objects whose type doesn't match any schema category
    const uncategorized: ObjectSummary[] = [];
    for (const [type, objs] of Object.entries(groupedObjects)) {
      if (!matchedTypes.has(type)) {
        uncategorized.push(...objs);
      }
    }

    if (uncategorized.length > 0) {
      items.push({
        type: 'header',
        category: { name: 'Uncategorized', icon: 'HelpCircle', color: 'warning' },
        count: uncategorized.length,
      });
      for (const obj of uncategorized) {
        items.push({ type: 'row', obj });
      }
    }

    return items;
  }, [groupedObjects, schema]);

  // Virtualizer — updated row heights for polish
  const totalItems = flatObjectItems.length;
  const rowHeight = isMobile ? 82 : 70;

  // eslint-disable-next-line react-hooks/incompatible-library
  const rowVirtualizer = useVirtualizer({
    count: totalItems,
    getScrollElement: () => parentRef.current,
    estimateSize: (index) => {
      const item = flatObjectItems[index];
      if (item?.type === 'header') return 28;
      if (item?.type === 'sub-header') return 24;
      return rowHeight;
    },
    overscan: 10,
  });

  // Sticky header logic
  const [scrollTop, setScrollTop] = useState(0);
  const [containerHeight, setContainerHeight] = useState(0);

  useEffect(() => {
    const el = parentRef.current;
    if (!el) return;

    setContainerHeight(el.clientHeight);

    const onScroll = () => setScrollTop(el.scrollTop);
    const onResize = () => setContainerHeight(el.clientHeight);

    el.addEventListener('scroll', onScroll, { passive: true });
    const ro = new ResizeObserver(onResize);
    ro.observe(el);

    return () => {
      el.removeEventListener('scroll', onScroll);
      ro.disconnect();
    };
  }, []);

  // Find selected item's index
  const selectedIndex = useMemo(() => {
    if (!selectedObject) return -1;
    return flatObjectItems.findIndex(
      (item) => item.type === 'row' && item.obj.id === selectedObject,
    );
  }, [selectedObject, flatObjectItems]);

  // Compute sticky position
  const stickyPosition = useMemo((): 'top' | 'bottom' | null => {
    if (selectedIndex < 0) return null;

    const allMeasurements = rowVirtualizer.measurementsCache;
    const itemMeasure = allMeasurements[selectedIndex];
    if (!itemMeasure) return null;

    const itemTop = itemMeasure.start;
    const itemBottom = itemMeasure.end;
    const viewTop = scrollTop;
    const viewBottom = scrollTop + containerHeight;

    if (itemBottom <= viewTop) return 'top';
    if (itemTop >= viewBottom) return 'bottom';
    return null;
  }, [selectedIndex, scrollTop, containerHeight, rowVirtualizer.measurementsCache]);

  const scrollToSelected = useCallback(() => {
    if (selectedIndex < 0) return;
    rowVirtualizer.scrollToIndex(selectedIndex, { align: 'center', behavior: 'smooth' });
  }, [selectedIndex, rowVirtualizer]);

  return {
    parentRef,
    rowVirtualizer,
    flatObjectItems,
    totalItems,
    stickyPosition,
    selectedIndex,
    scrollToSelected,
  };
}
