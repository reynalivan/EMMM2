import { useState, useCallback, type RefObject } from 'react';
import type { DragPosition } from '../../hooks/useFileDrop';
import { classifyDroppedPaths, validateDropForZone, type DropZone } from './dropUtils';
import type { DropValidation } from './DropConfirmModal';
import { scanService } from '../../lib/services/scanService';
import { toast } from '../../stores/useToastStore';
import type { ObjectSummary } from '../../types/object';

export interface UseObjectListDropZonesProps {
  activeGame: { id: string; name: string; game_type: string; mod_path: string } | null | undefined;
  objects: ObjectSummary[];
  toolbarRef: RefObject<HTMLDivElement | null>;
  contentRef: RefObject<HTMLDivElement | null>;
  bottomRef: RefObject<HTMLDivElement | null>;
  handleDropOnItem: (targetId: string, paths: string[]) => void;
  handleDropAutoOrganize: (paths: string[]) => void;
  setPendingPaths: (paths: string[]) => void;
  setCreateModalOpen: (open: boolean) => void;
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
  const [dropValidation, setDropValidation] = useState<DropValidation | null>(null);

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

  /** Pre-drop validation: score the drop against candidates, show modal if low confidence */
  const handleDropWithValidation = useCallback(
    async (paths: string[], position: DragPosition) => {
      if (!activeGame || !contentRef.current) return;

      // Resolve target object from position
      const element = document.elementFromPoint(position.x, position.y);
      let current: HTMLElement | null = element as HTMLElement;
      while (current && !current.dataset.objectId) {
        current = current.parentElement;
      }
      if (!current?.dataset.objectId) {
        toast.info('Drop on a specific object to move items there.');
        return;
      }

      const targetId = current.dataset.objectId;
      const targetObj = objects.find((o) => o.id === targetId);
      if (!targetObj) {
        toast.error('Target object not found.');
        return;
      }

      // Only validate folders (not loose files)
      const classified = classifyDroppedPaths(paths);
      const foldersToValidate = classified.folders;

      // If no folders, skip validation — just move directly
      if (foldersToValidate.length === 0) {
        handleDropOnItem(targetId, paths);
        return;
      }

      // Show validating modal
      setDropValidation({
        paths,
        targetId,
        targetName: targetObj.name,
        status: 'validating',
      });

      try {
        // Score the first dropped folder against all object names
        const candidateNames = objects.map((o) => o.name);
        const scores = await scanService.scoreCandidatesBatch(
          foldersToValidate[0],
          candidateNames,
          activeGame.game_type,
        );

        const targetScore = scores[targetObj.name] ?? 0;

        // Find best match
        let bestName = targetObj.name;
        let bestScore = targetScore;
        for (const [name, score] of Object.entries(scores)) {
          if (score > bestScore) {
            bestName = name;
            bestScore = score;
          }
        }

        const bestObj = objects.find((o) => o.name === bestName);

        // Confidence threshold: 50% or below → show warning
        if (targetScore <= 50) {
          setDropValidation({
            paths,
            targetId,
            targetName: targetObj.name,
            status: 'warning',
            targetScore,
            suggestedId: bestObj?.id,
            suggestedName: bestName,
            suggestedScore: bestScore,
          });
        } else {
          // High confidence — move directly
          setDropValidation(null);
          handleDropOnItem(targetId, paths);
        }
      } catch (e) {
        console.error('Pre-drop validation failed:', e);
        // On validation failure, move directly (fail-open)
        setDropValidation(null);
        handleDropOnItem(targetId, paths);
      }
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
        const el = document.elementFromPoint(pos.x, pos.y);
        let current: HTMLElement | null = el as HTMLElement;
        while (current && !current.dataset.objectId) {
          current = current.parentElement;
        }
        setHoveredItemId(current?.dataset.objectId ?? null);
        // Calculate tooltip Y relative to sidebar root
        const sidebarRect = contentRef.current?.parentElement?.getBoundingClientRect();
        setTooltipTop(sidebarRect ? pos.y - sidebarRect.top - 16 : pos.y);
      } else {
        setHoveredItemId(null);
      }
    },
    [resolveDropZone, contentRef],
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
    dropValidation,
    setDropValidation,
    onDrop,
    handleDragOver,
    handleDragStateChange,
    handleDropWithValidation,
  };
}
