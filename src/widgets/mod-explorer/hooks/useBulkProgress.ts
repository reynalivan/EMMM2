import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { isDemoMode } from '@/shared/lib/appMode';

export interface BulkProgressPayload {
  operation_id: string;
  cancellable: boolean;
  label: string;
  current: number;
  total: number;
  active: boolean;
}

export function useBulkProgress() {
  const [progress, setProgress] = useState<BulkProgressPayload>({
    operation_id: '',
    cancellable: false,
    label: '',
    current: 0,
    total: 0,
    active: false,
  });

  useEffect(() => {
    if (isDemoMode) {
      return;
    }

    let unlisten: () => void;

    const setupListener = async () => {
      unlisten = await listen<BulkProgressPayload>('bulk-progress', (event) => {
        setProgress(event.payload);

        // Auto-hide when complete
        if (event.payload.active && event.payload.current >= event.payload.total) {
          const completedOperationId = event.payload.operation_id;
          setTimeout(() => {
            setProgress((prev) =>
              prev.operation_id === completedOperationId ? { ...prev, active: false } : prev,
            );
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
