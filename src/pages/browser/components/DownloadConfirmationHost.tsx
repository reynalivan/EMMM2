import { useEffect, useState } from 'react';
import { createPortal } from 'react-dom';
import { listen } from '@tauri-apps/api/event';
import { useMutation } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { commands } from '@/shared/api/tauri/bindings';
import { useAppStore } from '@/app/store';
import { useBrowserStore } from '@/entities/browser';
import { toast } from '@/shared/ui/toast';
import type {
  DownloadConfirmationRequest,
  DownloadInformationFailure,
  DownloadInformationLoading,
} from '../types';
import { DownloadConfirmationDialog } from './DownloadConfirmationDialog';
import { DownloadInformationLoadingDialog } from './DownloadInformationLoadingDialog';

const DOWNLOAD_INFORMATION_TIMEOUT_MS = 10_000;

/**
 * App-level host for native WebView download confirmations. It stays mounted
 * while the user visits the Downloads workspace, where retry can also produce
 * a confirmation request.
 */
export function DownloadConfirmationHost() {
  const { t } = useTranslation(['browser']);
  const setWorkspaceView = useAppStore((state) => state.setWorkspaceView);
  const [requests, setRequests] = useState<DownloadConfirmationRequest[]>([]);
  const [loadingRequests, setLoadingRequests] = useState<DownloadInformationLoading[]>([]);
  const [informationFailures, setInformationFailures] = useState<DownloadInformationFailure[]>([]);
  const currentRequest = requests[0] ?? null;
  const currentLoadingRequest = loadingRequests[0] ?? null;
  const currentInformationFailure = informationFailures[0] ?? null;

  useEffect(() => {
    useBrowserStore
      .getState()
      .setDownloadConfirmationOpen(
        currentRequest !== null ||
          currentLoadingRequest !== null ||
          currentInformationFailure !== null,
      );
    return () => useBrowserStore.getState().setDownloadConfirmationOpen(false);
  }, [currentInformationFailure, currentLoadingRequest, currentRequest]);

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
    onError: (_error, request) => {
      setRequests((current) => current.filter((item) => item.id !== request.id));
      toast.error(t('downloads.confirmation.confirm_failed'));
    },
  });

  const rejectDownloadMutation = useMutation({
    mutationFn: (request: DownloadConfirmationRequest) =>
      commands.browserRejectDownload(request.id),
    onSuccess: (_result, request) => {
      setRequests((current) => current.filter((item) => item.id !== request.id));
    },
    onError: (_error, request) => {
      setRequests((current) => current.filter((item) => item.id !== request.id));
      toast.error(t('downloads.confirmation.cancel_failed'));
    },
  });

  useEffect(() => {
    const unlistenConfirmation = listen<DownloadConfirmationRequest>(
      'browser:download-confirmation-requested',
      (event) => {
        setInformationFailures((current) =>
          current.filter((request) => request.id !== event.payload.id),
        );
        setLoadingRequests((current) =>
          current.filter((request) => request.id !== event.payload.id),
        );
        setRequests((current) =>
          current.some((request) => request.id === event.payload.id)
            ? current
            : [...current, event.payload],
        );
      },
    );
    const unlistenLoading = listen<DownloadInformationLoading>(
      'browser:download-information-loading',
      (event) => {
        setLoadingRequests((current) =>
          current.some((request) => request.id === event.payload.id)
            ? current
            : [...current, event.payload],
        );
      },
    );
    const unlistenFailure = listen<DownloadInformationFailure>(
      'browser:download-information-failed',
      (event) => {
        setLoadingRequests((current) =>
          current.filter((request) => request.id !== event.payload.id),
        );
        setInformationFailures((current) =>
          current.some((request) => request.id === event.payload.id)
            ? current
            : [...current, event.payload],
        );
      },
    );

    return () => {
      unlistenConfirmation.then((unlisten) => unlisten());
      unlistenLoading.then((unlisten) => unlisten());
      unlistenFailure.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    const timers = loadingRequests.map((request) =>
      window.setTimeout(() => {
        setLoadingRequests((current) => current.filter((item) => item.id !== request.id));
        setInformationFailures((current) =>
          current.some((item) => item.id === request.id)
            ? current
            : [...current, { ...request, reason: 'timeout' }],
        );
      }, DOWNLOAD_INFORMATION_TIMEOUT_MS),
    );
    return () => timers.forEach((timer) => window.clearTimeout(timer));
  }, [loadingRequests]);

  if (!currentRequest && !currentLoadingRequest && !currentInformationFailure) return null;

  return createPortal(
    currentRequest ? (
      <DownloadConfirmationDialog
        key={currentRequest.id}
        request={currentRequest}
        isSubmitting={confirmDownloadMutation.isPending || rejectDownloadMutation.isPending}
        onConfirm={() => confirmDownloadMutation.mutate(currentRequest)}
        onReject={() => rejectDownloadMutation.mutate(currentRequest)}
      />
    ) : (
      <DownloadInformationLoadingDialog
        request={currentInformationFailure ?? currentLoadingRequest!}
        failure={currentInformationFailure}
        onDismiss={() =>
          setInformationFailures((current) =>
            currentInformationFailure
              ? current.filter((request) => request.id !== currentInformationFailure.id)
              : current,
          )
        }
      />
    ),
    document.body,
  );
}
