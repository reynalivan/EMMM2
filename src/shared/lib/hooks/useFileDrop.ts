import { useEffect, useState, useRef } from 'react';
import { listen, type EventCallback, type UnlistenFn } from '@tauri-apps/api/event';
import { isDemoMode } from '@/shared/lib/appMode';
import { reportFrontendError } from '@/shared/lib/telemetry';

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
   * Source validation is performed by the receiving import flow.
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
    if (!enabled || isDemoMode) return;

    let disposed = false;
    const unlisteners: UnlistenFn[] = [];
    const dispose = () => {
      disposed = true;
      for (const unlisten of unlisteners.splice(0)) unlisten();
    };
    const register = <T>(name: string, handler: EventCallback<T>) =>
      listen<T>(name, (event) => {
        if (!disposed) handler(event);
      }).then((unlisten) => {
        if (disposed) unlisten();
        else unlisteners.push(unlisten);
      });

    /** Tauri reports physical pixels; elementFromPoint needs CSS pixels. */
    const toLogical = (pos: { x: number; y: number }): DragPosition => {
      const dpr = window.devicePixelRatio || 1;
      return { x: pos.x / dpr, y: pos.y / dpr };
    };

    void Promise.all([
      register<FileDropPayload>('tauri://drag-enter', (event) => {
        setIsDragging(true);
        onDragStateChangeRef.current?.(true);
        if (event.payload.position) {
          const pos = toLogical(event.payload.position);
          setDragPosition(pos);
          onDragOverRef.current?.(pos);
        }
      }),
      register<FileDropPayload>('tauri://drag-over', (event) => {
        const pos = toLogical(event.payload.position);
        setDragPosition(pos);
        onDragOverRef.current?.(pos);
      }),
      register('tauri://drag-leave', () => {
        setIsDragging(false);
        setDragPosition(null);
        onDragStateChangeRef.current?.(false);
      }),
      register<FileDropPayload>('tauri://drag-drop', (event) => {
        setIsDragging(false);
        setDragPosition(null);
        onDragStateChangeRef.current?.(false);
        const pos = toLogical(event.payload.position ?? { x: 0, y: 0 });

        if (event.payload.paths && event.payload.paths.length > 0) {
          onDropRef.current(event.payload.paths, pos);
        }
      }),
    ]).catch((error: unknown) => {
      const report = !disposed;
      dispose();
      if (report) reportFrontendError(error, 'unhandled_rejection', { promptUser: true });
    });

    return dispose;
  }, [enabled]);

  return { isDragging, dragPosition };
}
