import { useEffect, useState, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { classifyDroppedPaths, allUnsupported } from '../features/sidebar/dropUtils';
import { toast } from '../stores/useToastStore';

interface FileDropPayload {
  paths: string[];
  position: { x: number; y: number };
}

export interface DragPosition {
  x: number;
  y: number;
}

interface UseFileDropProps {
  /**
   * Called when files are dropped. Receives paths and cursor position.
   * Unsupported files are auto-rejected with a toast before this fires.
   */
  onDrop: (paths: string[], position: DragPosition) => void;
  /** Called on every drag-over with the current cursor position. */
  onDragOver?: (position: DragPosition) => void;
  /** Called when dragging starts/stops. */
  onDragStateChange?: (dragging: boolean) => void;
  enabled?: boolean;
}

export function useFileDrop({
  onDrop,
  onDragOver,
  onDragStateChange,
  enabled = true,
}: UseFileDropProps) {
  const [isDragging, setIsDragging] = useState(false);
  const [dragPosition, setDragPosition] = useState<DragPosition | null>(null);

  // Use refs to avoid re-registering listeners on every callback change
  const onDropRef = useRef(onDrop);
  const onDragOverRef = useRef(onDragOver);
  const onDragStateChangeRef = useRef(onDragStateChange);
  useEffect(() => {
    onDropRef.current = onDrop;
    onDragOverRef.current = onDragOver;
    onDragStateChangeRef.current = onDragStateChange;
  }, [onDrop, onDragOver, onDragStateChange]);

  useEffect(() => {
    if (!enabled) return;

    let unlistenDrop: (() => void) | undefined;
    let unlistenEnter: (() => void) | undefined;
    let unlistenLeave: (() => void) | undefined;
    let unlistenOver: (() => void) | undefined;

    const setupListeners = async () => {
      /** Tauri reports physical pixels; elementFromPoint needs CSS pixels */
      const toLogical = (pos: { x: number; y: number }): DragPosition => {
        const dpr = window.devicePixelRatio || 1;
        return { x: pos.x / dpr, y: pos.y / dpr };
      };

      unlistenEnter = await listen<FileDropPayload>('tauri://drag-enter', (event) => {
        setIsDragging(true);
        onDragStateChangeRef.current?.(true);
        if (event.payload.position) {
          const pos = toLogical(event.payload.position);
          setDragPosition(pos);
          onDragOverRef.current?.(pos);
        }
      });

      unlistenOver = await listen<FileDropPayload>('tauri://drag-over', (event) => {
        const pos = toLogical(event.payload.position);
        setDragPosition(pos);
        onDragOverRef.current?.(pos);
      });

      unlistenLeave = await listen('tauri://drag-leave', () => {
        setIsDragging(false);
        setDragPosition(null);
        onDragStateChangeRef.current?.(false);
      });

      unlistenDrop = await listen<FileDropPayload>('tauri://drag-drop', (event) => {
        setIsDragging(false);
        setDragPosition(null);
        onDragStateChangeRef.current?.(false);
        const pos = toLogical(event.payload.position ?? { x: 0, y: 0 });

        if (event.payload.paths && event.payload.paths.length > 0) {
          const classified = classifyDroppedPaths(event.payload.paths);

          // Reject if ALL files are unsupported
          if (allUnsupported(classified)) {
            toast.error(
              'Unsupported file type. Accepted: folders, archives (.zip/.rar/.7z), .ini, images.',
            );
            return;
          }

          // Warn about skipped unsupported files (but still process valid ones)
          if (classified.unsupported.length > 0) {
            toast.warning(`Skipped ${classified.unsupported.length} unsupported file(s).`);
          }

          onDropRef.current(event.payload.paths, pos);
        }
      });
    };

    setupListeners();

    return () => {
      unlistenEnter?.();
      unlistenLeave?.();
      unlistenDrop?.();
      unlistenOver?.();
    };
  }, [enabled]);

  return { isDragging, dragPosition };
}
