import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';

export interface BulkProgressPayload {
  label: string;
  current: number;
  total: number;
  active: boolean;
}

export function useBulkProgress() {
  const [progress, setProgress] = useState<BulkProgressPayload>({
    label: '',
    current: 0,
    total: 0,
    active: false,
  });

  useEffect(() => {
    let unlisten: () => void;

    const setupListener = async () => {
      unlisten = await listen<BulkProgressPayload>('bulk-progress', (event) => {
        setProgress(event.payload);

        // Auto-hide when complete
        if (event.payload.active && event.payload.current >= event.payload.total) {
          setTimeout(() => {
            setProgress((prev) => ({ ...prev, active: false }));
          }, 1500);
        }
      });
    };

    setupListener();

    return () => {
      unlisten?.();
    };
  }, []);

  return progress;
}
