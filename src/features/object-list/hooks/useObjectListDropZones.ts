import { useState, useCallback, type RefObject } from 'react';
import type { DragPosition } from '../../../hooks/useFileDrop';
import { classifyDroppedPaths, validateDropForZone, type DropZone } from '../utils/dropUtils';

export type { DropZone } from '../utils/dropUtils';
import { toast } from '../../../stores/useToastStore';
import type { GameType } from '../../../types/game';
import type { WorkspaceObjectNode } from '../../../types/workspace';

export interface UseObjectListDropZonesProps {
  activeGame:
    { id: string; name: string; game_type: GameType; mod_path: string } | null | undefined;
  objects: WorkspaceObjectNode[];
  toolbarRef: RefObject<HTMLDivElement | null>;
  contentRef: RefObject<HTMLDivElement | null>;
  bottomRef: RefObject<HTMLDivElement | null>;
  handleDropOnItem: (targetId: string, paths: string[]) => void;
  handleDropAutoOrganize: (paths: string[]) => void;
  setPendingPaths: (paths: string[]) => void;
  setCreateModalOpen: (open: boolean) => void;
}

/** Nearest ancestor of the point that carries a data-object-id. */
function findObjectIdAtPoint(position: DragPosition): string | null {
  let current = document.elementFromPoint(position.x, position.y) as HTMLElement | null;
  while (current && !current.dataset.objectId) {
    current = current.parentElement;
  }
  return current?.dataset.objectId ?? null;
}

export function useObjectListDropZones({
  activeGame,
  objects,
  toolbarRef,
  contentRef,
  bottomRef,
  handleDropOnItem,
  handleDropAutoOrganize,
  setPendingPaths,
  setCreateModalOpen,
}: UseObjectListDropZonesProps) {
  const [activeDropZone, setActiveDropZone] = useState<DropZone | null>(null);
  const [hoveredItemId, setHoveredItemId] = useState<string | null>(null);
  const [tooltipTop, setTooltipTop] = useState<number>(0);

  /** Resolve which drop zone the cursor is in */
  const resolveDropZone = useCallback(
    (position: DragPosition): DropZone | null => {
      const toolbarEl = toolbarRef.current;
      const bottomEl = bottomRef.current;

      if (toolbarEl) {
        const rect = toolbarEl.getBoundingClientRect();
        if (
          position.x >= rect.left &&
          position.x <= rect.right &&
          position.y >= rect.top &&
          position.y <= rect.bottom
        ) {
          return 'auto-organize';
        }
      }

      if (bottomEl) {
        const rect = bottomEl.getBoundingClientRect();
        if (
          position.x >= rect.left &&
          position.x <= rect.right &&
          position.y >= rect.top &&
          position.y <= rect.bottom
        ) {
          return 'new-object';
        }
      }

      // Default to item zone if within content area
      if (contentRef.current) {
        const rect = contentRef.current.getBoundingClientRect();
        if (
          position.x >= rect.left &&
          position.x <= rect.right &&
          position.y >= rect.top &&
          position.y <= rect.bottom
        ) {
          return 'item';
        }
      }

      return null;
    },
    [toolbarRef, bottomRef, contentRef],
  );

  /** Resolve a specific-target drop. Match Wizard performs the actual validation. */
  const handleDropWithValidation = useCallback(
    (paths: string[], position: DragPosition) => {
      if (!activeGame || !contentRef.current) return;

      const targetId = findObjectIdAtPoint(position);
      if (!targetId) {
        toast.info('Drop on a specific object to move items there.');
        return;
      }

      const targetObject = objects.find((object) => object.id === targetId);
      if (!targetObject) {
        toast.error('Target object not found.');
        return;
      }
      if (!targetObject.is_registered) {
        toast.info('Register this folder before moving mods into it.');
        return;
      }
      handleDropOnItem(targetId, paths);
    },
    [activeGame, objects, handleDropOnItem, contentRef],
  );

  // US-3.Z: Zone-aware DnD handler
  const onDrop = useCallback(
    (paths: string[], position: DragPosition) => {
      if (!activeGame || paths.length === 0) return;

      const zone = resolveDropZone(position);
      if (!zone) {
        toast.info('Drop inside a zone to import items.');
        return;
      }

      const classified = classifyDroppedPaths(paths);
      const validation = validateDropForZone(zone, classified);

      if (!validation.valid) {
        toast.error(validation.reason ?? 'Invalid drop');
        return;
      }

      switch (zone) {
        case 'auto-organize':
          handleDropAutoOrganize(paths);
          break;
        case 'item':
          handleDropWithValidation(paths, position);
          break;
        case 'new-object':
          setPendingPaths(paths);
          setCreateModalOpen(true);
          break;
      }
    },
    [
      activeGame,
      resolveDropZone,
      handleDropAutoOrganize,
      handleDropWithValidation,
      setPendingPaths,
      setCreateModalOpen,
    ],
  );

  // Zone detection via onDragOver callback (React-compliant: setState from event handler)
  const handleDragOver = useCallback(
    (pos: DragPosition) => {
      const zone = resolveDropZone(pos);
      setActiveDropZone(zone);

      // Track which object row the cursor is over (for per-item highlight)
      if (zone === 'item') {
        const hoveredId = findObjectIdAtPoint(pos);
        const registeredId = objects.some(
          (object) => object.id === hoveredId && object.is_registered,
        )
          ? hoveredId
          : null;
        setHoveredItemId(registeredId);
        // Calculate tooltip Y relative to sidebar root
        const sidebarRect = contentRef.current?.parentElement?.getBoundingClientRect();
        setTooltipTop(sidebarRect ? pos.y - sidebarRect.top - 16 : pos.y);
      } else {
        setHoveredItemId(null);
      }
    },
    [resolveDropZone, contentRef, objects],
  );

  const handleDragStateChange = useCallback((dragging: boolean) => {
    if (!dragging) {
      setActiveDropZone(null);
      setHoveredItemId(null);
      setTooltipTop(0);
    }
  }, []);

  return {
    activeDropZone,
    hoveredItemId,
    tooltipTop,
    onDrop,
    handleDragOver,
    handleDragStateChange,
  };
}
