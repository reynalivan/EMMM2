import { useState } from 'react';
import { createPortal } from 'react-dom';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { CircleHelp, FolderSearch2, LoaderCircle, ScanSearch } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { commands } from '@/shared/api/tauri/bindings';
import type { ObjectIdentitySuggestionStatus } from '@/shared/api/tauri/bindings.gen';
import { publishQueryInvalidations } from '@/shared/lib/queryRefresh';
import { openObjectClassificationWizard } from '@/features/import-batches';

const identitySuggestionKeys = {
  status: (gameId: string) => ['object-identity-suggestions', gameId] as const,
};

type Props = {
  gameId: string | null;
  onOpenSettings: () => void;
};

export function DashboardIdentitySuggestions({ gameId, onOpenSettings }: Props) {
  const { t } = useTranslation('dashboard');
  const queryClient = useQueryClient();
  const [dismissedForSession, setDismissedForSession] = useState<string | null>(null);
  const [reviewing, setReviewing] = useState(false);
  const [isHowToOpen, setIsHowToOpen] = useState(false);
  const query = useQuery<ObjectIdentitySuggestionStatus>({
    queryKey: identitySuggestionKeys.status(gameId ?? 'none'),
    queryFn: () => commands.getObjectIdentitySuggestionStatus(gameId ?? ''),
    enabled: Boolean(gameId),
    staleTime: 2_000,
    refetchInterval: (current) => (current.state.data?.state === 'checking' ? 1_000 : false),
  });

  if (!gameId || dismissedForSession === gameId || !query.data) return null;
  const status = query.data;
  const invalidate = () =>
    publishQueryInvalidations(queryClient, [identitySuggestionKeys.status(gameId)], 'active');
  const check = async () => {
    await commands.retryObjectIdentitySuggestions(gameId, null);
    await invalidate();
  };
  const review = async () => {
    setReviewing(true);
    try {
      const objectIds: string[] = [];
      let offset: number | null = 0;
      while (offset !== null) {
        const page = await commands.listObjectIdentitySuggestions(gameId, offset, 100);
        objectIds.push(...page.items.map((item) => item.objectId));
        if (page.nextOffset !== null && page.nextOffset <= offset) {
          throw new Error('Suggestion pagination did not advance');
        }
        offset = page.nextOffset;
      }
      if (objectIds.length === 0) {
        await invalidate();
        return;
      }
      openObjectClassificationWizard({
        gameId,
        objectIds,
        onComplete: () => void invalidate(),
        onIgnore: async (objectId) => {
          await commands.dismissObjectIdentitySuggestion(gameId, objectId);
          await invalidate();
        },
      });
    } finally {
      setReviewing(false);
    }
  };

  const supportingText = status.hasKeyviewerTargets
    ? t('identity_suggestions.benefit_keyviewer')
    : t('identity_suggestions.benefit_matching');
  const title =
    status.state === 'catalog_missing'
      ? t('identity_suggestions.setup_title')
      : status.state === 'unsupported'
        ? t('identity_suggestions.unsupported_title')
        : status.state === 'checking'
          ? t('identity_suggestions.checking_title')
          : status.state === 'error'
            ? t('identity_suggestions.error_title')
            : status.suggestedCount > 0
              ? t('identity_suggestions.ready_title', { count: status.suggestedCount })
              : t('identity_suggestions.check_title');
  const detail =
    status.state === 'checking'
      ? t('identity_suggestions.checking_detail', {
          checked: status.checkedCount,
          total: status.totalCount,
        })
      : status.state === 'catalog_missing'
        ? t('identity_suggestions.setup_detail')
        : status.state === 'unsupported'
          ? t('identity_suggestions.unsupported_detail')
          : status.state === 'error'
            ? status.message || t('identity_suggestions.error_detail')
            : status.suggestedCount > 0
              ? supportingText
              : t('identity_suggestions.check_detail');

  const howToDialog = isHowToOpen ? (
    <dialog
      aria-describedby="identity-suggestions-how-to-description"
      aria-labelledby="identity-suggestions-how-to-title"
      className="modal modal-open modal-middle p-4"
      onCancel={(event) => {
        event.preventDefault();
        setIsHowToOpen(false);
      }}
      onClose={() => setIsHowToOpen(false)}
      open
    >
      <div className="modal-box max-h-[calc(100dvh-2rem)] w-11/12 max-w-lg overflow-hidden p-5">
        <h2 id="identity-suggestions-how-to-title" className="text-lg font-semibold">
          {t('identity_suggestions.how_to_title')}
        </h2>
        <p
          id="identity-suggestions-how-to-description"
          className="mt-2 text-sm leading-6 text-base-content/65"
        >
          {t('identity_suggestions.how_to_intro')}
        </p>
        <div className="mt-4 text-sm">
          <h3 className="font-medium">{t('identity_suggestions.how_to_benefits_title')}</h3>
          <ul className="mt-2 list-disc space-y-2 pl-5 text-base-content/70">
            <li>{t('identity_suggestions.how_to_benefit_matching')}</li>
            <li>{t('identity_suggestions.how_to_benefit_categories')}</li>
            <li>
              {status.hasKeyviewerTargets
                ? t('identity_suggestions.how_to_benefit_keyviewer')
                : t('identity_suggestions.how_to_benefit_safe')}
            </li>
          </ul>
        </div>
        <div className="modal-action">
          {status.state === 'catalog_missing' ? (
            <button
              className="btn btn-primary"
              type="button"
              onClick={() => {
                setIsHowToOpen(false);
                onOpenSettings();
              }}
            >
              {t('identity_suggestions.setup_action')}
            </button>
          ) : status.state !== 'unsupported' && status.state !== 'checking' ? (
            <button
              className="btn btn-primary"
              type="button"
              onClick={() => {
                setIsHowToOpen(false);
                void check();
              }}
            >
              {t('identity_suggestions.check_action')}
            </button>
          ) : null}
          <button className="btn btn-ghost" type="button" onClick={() => setIsHowToOpen(false)}>
            {t('identity_suggestions.how_to_close')}
          </button>
        </div>
      </div>
      <form className="modal-backdrop" method="dialog">
        <button type="button" onClick={() => setIsHowToOpen(false)}>
          {t('identity_suggestions.how_to_close')}
        </button>
      </form>
    </dialog>
  ) : null;
  const overlayRoot =
    typeof document === 'undefined'
      ? null
      : (document.getElementById('workspace-main') ?? document.body);

  return (
    <>
      <section
        className="dashboard-section-enter rounded-box border border-base-300 bg-base-200/50 px-4 py-3"
        aria-live="polite"
      >
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex min-w-0 items-start gap-3">
            {status.state === 'checking' ? (
              <LoaderCircle className="mt-0.5 shrink-0 animate-spin text-primary" size={18} />
            ) : (
              <ScanSearch className="mt-0.5 shrink-0 text-primary" size={18} />
            )}
            <div>
              <h2 className="font-semibold">{title}</h2>
              <p className="mt-0.5 text-sm text-base-content/60">{detail}</p>
            </div>
          </div>
          <div className="flex shrink-0 flex-wrap items-center gap-2">
            {status.state === 'catalog_missing' ? (
              <button className="btn btn-sm btn-primary" type="button" onClick={onOpenSettings}>
                <FolderSearch2 size={15} /> {t('identity_suggestions.setup_action')}
              </button>
            ) : status.state === 'unsupported' ||
              status.state === 'checking' ? null : status.suggestedCount > 0 ? (
              <>
                <button
                  className="btn btn-sm btn-primary"
                  type="button"
                  disabled={reviewing}
                  onClick={() => void review()}
                >
                  {reviewing ? (
                    <LoaderCircle className="animate-spin" size={15} />
                  ) : (
                    <ScanSearch size={15} />
                  )}
                  {t('identity_suggestions.review_action', { count: status.suggestedCount })}
                </button>
                <button
                  className="btn btn-sm btn-ghost"
                  type="button"
                  onClick={() => setDismissedForSession(gameId)}
                >
                  {t('identity_suggestions.not_now')}
                </button>
              </>
            ) : (
              <button className="btn btn-sm btn-primary" type="button" onClick={() => void check()}>
                <ScanSearch size={15} /> {t('identity_suggestions.check_action')}
              </button>
            )}
            <button
              className="btn btn-sm btn-ghost btn-square"
              type="button"
              aria-label={t('identity_suggestions.how_to_action')}
              title={t('identity_suggestions.how_to_action')}
              onClick={() => setIsHowToOpen(true)}
            >
              <CircleHelp size={16} />
            </button>
          </div>
        </div>
      </section>
      {howToDialog && overlayRoot ? createPortal(howToDialog, overlayRoot) : null}
    </>
  );
}
