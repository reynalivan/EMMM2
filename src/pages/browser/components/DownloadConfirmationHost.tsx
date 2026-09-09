import { useEffect, useState } from 'react';
import { createPortal } from 'react-dom';
import { listen } from '@tauri-apps/api/event';
import { useMutation } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { commands } from '@/shared/api/tauri/bindings';
import { useAppStore } from '@/app/store';
import { useBrowserStore } from '@/entities/browser';
import { toast } from '@/shared/ui/toast';
import type { DownloadConfirmationRequest } from '../types';
import { DownloadConfirmationDialog } from './DownloadConfirmationDialog';

/**
 * App-level host for native WebView download confirmations. It stays mounted
 * while the user visits the Downloads workspace, where retry can also produce
 * a confirmation request.
 */
export function DownloadConfirmationHost() {
  const { t } = useTranslation(['browser']);
  const setWorkspaceView = useAppStore((state) => state.setWorkspaceView);
  const [requests, setRequests] = useState<DownloadConfirmationRequest[]>([]);
  const currentRequest = requests[0] ?? null;

  useEffect(() => {
    useBrowserStore.getState().setDownloadConfirmationOpen(currentRequest !== null);
    return () => useBrowserStore.getState().setDownloadConfirmationOpen(false);
  }, [currentRequest]);

  const confirmDownloadMutation = useMutation({
    mutationFn: (request: DownloadConfirmationRequest) =>
      commands.browserConfirmDownload(request.id),
    onSuccess: (_result, request) => {
      setRequests((current) => current.filter((item) => item.id !== request.id));
      toast.withAction('info', t('downloads.feedback.queued', { filename: request.filename }), {
        label: t('downloads.view_detail'),
        onClick: () => setWorkspaceView('downloads'),
      });
    },
    onError: () => toast.error(t('downloads.confirmation.confirm_failed')),
  });

  const rejectDownloadMutation = useMutation({
    mutationFn: (request: DownloadConfirmationRequest) =>
      commands.browserRejectDownload(request.id),
    onSuccess: (_result, request) => {
      setRequests((current) => current.filter((item) => item.id !== request.id));
    },
    onError: () => toast.error(t('downloads.confirmation.cancel_failed')),
  });

  useEffect(() => {
    const unlistenConfirmation = listen<DownloadConfirmationRequest>(
      'browser:download-confirmation-requested',
      (event) => {
        setRequests((current) =>
          current.some((request) => request.id === event.payload.id)
            ? current
            : [...current, event.payload],
        );
      },
    );

    return () => {
      unlistenConfirmation.then((unlisten) => unlisten());
    };
  }, []);

  if (!currentRequest) return null;

  return createPortal(
    <DownloadConfirmationDialog
      key={currentRequest.id}
      request={currentRequest}
      isSubmitting={confirmDownloadMutation.isPending || rejectDownloadMutation.isPending}
      onConfirm={() => confirmDownloadMutation.mutate(currentRequest)}
      onReject={() => rejectDownloadMutation.mutate(currentRequest)}
    />,
    document.body,
  );
}
