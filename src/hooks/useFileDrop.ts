import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';

interface FileDropPayload {
  paths: string[];
  position: { x: number; y: number };
}

interface UseFileDropProps {
  onDrop: (paths: string[]) => void;
  enabled?: boolean;
}

export function useFileDrop({ onDrop, enabled = true }: UseFileDropProps) {
  const [isDragging, setIsDragging] = useState(false);

  useEffect(() => {
    if (!enabled) return;

    let unlistenDrop: () => void;
    let unlistenEnter: () => void;
    let unlistenLeave: () => void; // Or handle via enter/leave count? No, simpler is direct.
    // Tauri events are global for the window.

    // Using a counter or state to handle enter/leave correctly?
    // Actually, tauri drag events are simple. Enter fires once when entering window. Leave when leaving window.

    const setupListeners = async () => {
      unlistenEnter = await listen('tauri://drag-enter', () => {
        setIsDragging(true);
      });

      unlistenLeave = await listen('tauri://drag-leave', () => {
        setIsDragging(false);
      });

      unlistenDrop = await listen<FileDropPayload>('tauri://drag-drop', (event) => {
        setIsDragging(false);
        if (event.payload.paths && event.payload.paths.length > 0) {
          onDrop(event.payload.paths);
        }
      });
    };

    setupListeners();

    return () => {
      if (unlistenEnter) unlistenEnter();
      if (unlistenLeave) unlistenLeave();
      if (unlistenDrop) unlistenDrop();
    };
  }, [enabled, onDrop]);

  return { isDragging };
}
