import { Pencil, Trash2, Image as ImageIcon, Info, FolderOpen, File, Loader2 } from 'lucide-react';
import { createPortal } from 'react-dom';
import {
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type RefObject,
} from 'react';
import { useTranslation } from 'react-i18next';
import { convertFileSrc } from '@tauri-apps/api/core';
import type {
  FolderConflictSummary,
  FolderNameConflictCandidate,
} from '../../../shared/api/tauri/bindings';
import { formatBytes } from '../../../shared/lib/utils/formatters';
import type { FolderConflictCandidateAction } from './folderConflictDrafts';

interface Props {
  candidate: FolderNameConflictCandidate;
  detail?: FolderConflictSummary;
  isKeep: boolean;
  action: FolderConflictCandidateAction;
  value: string;
  error?: string;
  disabled?: boolean;
  inputRef: (element: HTMLInputElement | null) => void;
  onKeep: () => void;
  onActionChange: (action: FolderConflictCandidateAction) => void;
  onChange: (value: string) => void;
  onBlur: () => void;
  onOpenFolder: () => Promise<void>;
}

interface FolderConflictDetailPanelProps {
  candidate: FolderNameConflictCandidate;
  detail: FolderConflictSummary;
  panelId: string;
  panelRef: RefObject<HTMLDivElement | null>;
  position: { top: number; left: number };
}

function FolderConflictDetailPanel({
  candidate,
  detail,
  panelId,
  panelRef,
  position,
}: FolderConflictDetailPanelProps) {
  const { t } = useTranslation('folder_grid');

  return (
    <div
      ref={panelRef}
      id={panelId}
      role="dialog"
      aria-label={t('conflict_manager.folder_info')}
      className="fixed z-[1000] max-h-[calc(100vh-2rem)] w-[min(18rem,calc(100vw-2rem))] overflow-y-auto rounded-xl border border-base-content/10 bg-base-100 p-0 text-xs shadow-2xl"
      style={{ top: position.top, left: position.left }}
      onClick={(event) => event.stopPropagation()}
    >
      <div className="border-b border-base-content/5 bg-base-200/50 p-3">
        <h4 className="mb-2.5 flex items-center gap-2 font-semibold text-base-content">
          <FolderOpen size={14} className="text-base-content/70" />
          {t('conflict_manager.folder_info')}
        </h4>
        <div className="grid grid-cols-2 gap-3 text-[11px]">
          <div>
            <span className="mb-0.5 block text-[9px] font-semibold uppercase tracking-wider text-base-content/40">
              {t('conflict_manager.created')}
            </span>
            <span className="font-medium">
              {detail.created_at
                ? new Date(detail.created_at).toLocaleDateString(undefined, {
                    year: 'numeric',
                    month: 'short',
                    day: 'numeric',
                  })
                : '-'}
            </span>
          </div>
          <div>
            <span className="mb-0.5 block text-[9px] font-semibold uppercase tracking-wider text-base-content/40">
              {t('conflict_manager.modified')}
            </span>
            <span className="font-medium">
              {detail.modified_at
                ? new Date(detail.modified_at).toLocaleDateString(undefined, {
                    year: 'numeric',
                    month: 'short',
                    day: 'numeric',
                  })
                : '-'}
            </span>
          </div>
        </div>
      </div>

      <div className="p-3">
        <div className="mb-2.5 flex items-center justify-between text-[9px] font-bold uppercase tracking-wider text-base-content/40">
          <span>{t('conflict_manager.contents', { count: detail.file_count })}</span>
        </div>
        <div className="max-h-40 space-y-2 overflow-y-auto pr-1">
          {detail.files.map((file, index) => (
            <div key={index} className="flex items-start gap-2 text-[11px] text-base-content/80">
              <File size={12} className="mt-0.5 shrink-0 opacity-40" />
              <span className="truncate leading-tight" title={file}>
                {file}
              </span>
            </div>
          ))}
          {detail.file_count > detail.files.length && (
            <div className="mt-1 pl-5 text-[10px] italic text-base-content/40">
              {t('conflict_manager.more_files', {
                count: detail.file_count - detail.files.length,
              })}
            </div>
          )}
        </div>
      </div>

      <div className="border-t border-base-content/5 bg-base-200/30 p-3">
        <div className="mb-1.5 text-[9px] font-bold uppercase tracking-wider text-base-content/40">
          {t('conflict_manager.full_path')}
        </div>
        <div className="break-all font-mono text-[10px] leading-relaxed text-base-content/60">
          {candidate.path}
        </div>
      </div>
    </div>
  );
}

export default function FolderConflictCandidateCard({
  candidate,
  detail,
  isKeep,
  action,
  value,
  error,
  disabled = false,
  inputRef,
  onKeep,
  onActionChange,
  onChange,
  onBlur,
  onOpenFolder,
}: Props) {
  const { t } = useTranslation('folder_grid');
  const detailPanelId = useId();
  const infoTriggerRef = useRef<HTMLButtonElement>(null);
  const infoPanelRef = useRef<HTMLDivElement>(null);
  const [isInfoOpen, setIsInfoOpen] = useState(false);
  const [infoPortalTarget, setInfoPortalTarget] = useState<HTMLElement | null>(null);
  const [infoPosition, setInfoPosition] = useState({ top: 16, left: 16 });
  const [isOpeningFolder, setIsOpeningFolder] = useState(false);

  const updateInfoPosition = useCallback(() => {
    const trigger = infoTriggerRef.current;
    if (!trigger) return;

    const triggerRect = trigger.getBoundingClientRect();
    const panelHeight = infoPanelRef.current?.getBoundingClientRect().height ?? 320;
    const panelWidth = Math.min(288, Math.max(0, window.innerWidth - 32));
    const gap = 8;
    const opensAbove = triggerRect.bottom + panelHeight + gap > window.innerHeight - 16;
    const maxTop = Math.max(16, window.innerHeight - panelHeight - 16);
    const maxLeft = Math.max(16, window.innerWidth - panelWidth - 16);

    setInfoPosition({
      top: opensAbove
        ? Math.max(16, triggerRect.top - panelHeight - gap)
        : Math.min(maxTop, triggerRect.bottom + gap),
      left: Math.min(maxLeft, Math.max(16, triggerRect.left)),
    });
  }, []);

  useLayoutEffect(() => {
    if (isInfoOpen) updateInfoPosition();
  }, [isInfoOpen, updateInfoPosition]);

  useEffect(() => {
    if (!isInfoOpen) return;

    const handleOutsidePointerDown = (event: PointerEvent) => {
      const target = event.target;
      if (!(target instanceof Node)) return;
      if (infoTriggerRef.current?.contains(target) || infoPanelRef.current?.contains(target))
        return;
      setIsInfoOpen(false);
    };
    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setIsInfoOpen(false);
    };

    document.addEventListener('pointerdown', handleOutsidePointerDown, true);
    document.addEventListener('keydown', handleEscape);
    window.addEventListener('resize', updateInfoPosition);
    window.addEventListener('scroll', updateInfoPosition, true);

    return () => {
      document.removeEventListener('pointerdown', handleOutsidePointerDown, true);
      document.removeEventListener('keydown', handleEscape);
      window.removeEventListener('resize', updateInfoPosition);
      window.removeEventListener('scroll', updateInfoPosition, true);
    };
  }, [isInfoOpen, updateInfoPosition]);

  return (
    <>
      <article
        className={`rounded-lg border p-3.5 transition-colors ${
          isKeep
            ? 'border-success/40 bg-success/5 border-2'
            : action === 'trash'
              ? 'border-error/30 bg-error/5 hover:bg-error/10'
              : 'border-base-content/10 bg-base-200/30 hover:bg-base-200/60'
        }`}
      >
        <div
          className={`flex items-start gap-3 ${!isKeep ? 'cursor-pointer' : ''}`}
          onClick={!isKeep ? onKeep : undefined}
        >
          <div className="mt-0.5 shrink-0">
            <input
              type="radio"
              className={`radio radio-sm ${isKeep ? 'radio-success' : 'radio-neutral/40'}`}
              checked={isKeep}
              readOnly
              disabled={disabled}
              aria-label={t('conflict_manager.keep_instead')}
            />
          </div>

          <div className="shrink-0 w-12 h-12 rounded-lg bg-base-300/50 border border-base-content/10 overflow-hidden flex items-center justify-center shadow-sm">
            {detail?.thumbnail_path ? (
              <img
                src={convertFileSrc(detail.thumbnail_path)}
                alt=""
                className="w-full h-full object-cover"
              />
            ) : (
              <ImageIcon size={20} className="text-base-content/20" />
            )}
          </div>

          <div className="flex-1 min-w-0 flex flex-col justify-center min-h-[3rem]">
            <div className="mb-1 flex min-w-0 items-center gap-2">
              <h3
                className="min-w-0 flex-1 truncate text-sm font-semibold"
                title={candidate.folder_name}
              >
                {candidate.folder_name}
              </h3>
              <span
                className={`badge badge-xs ${candidate.is_enabled ? 'badge-success' : 'badge-neutral'}`}
              >
                {candidate.is_enabled
                  ? t('conflict_manager.enabled')
                  : t('conflict_manager.disabled')}
              </span>
              {isKeep && (
                <span className="badge badge-xs badge-success ml-auto shadow-sm">
                  {t('conflict_manager.keep_badge')}
                </span>
              )}
              <button
                type="button"
                className="btn btn-ghost btn-xs btn-square ml-1 shrink-0"
                aria-label={t('conflict_manager.open_folder')}
                title={t('conflict_manager.open_folder')}
                disabled={disabled || isOpeningFolder}
                onClick={(event) => {
                  event.stopPropagation();
                  setIsOpeningFolder(true);
                  void onOpenFolder().finally(() => setIsOpeningFolder(false));
                }}
              >
                {isOpeningFolder ? (
                  <Loader2 size={14} className="animate-spin motion-reduce:animate-none" />
                ) : (
                  <FolderOpen size={14} />
                )}
              </button>
            </div>

            <p className="mb-1 break-all font-mono text-[10px] leading-relaxed text-base-content/50">
              <span className="sr-only">{t('conflict_manager.full_path')}: </span>
              {candidate.path}
            </p>

            {detail ? (
              <button
                ref={infoTriggerRef}
                type="button"
                className="inline-flex items-center gap-1.5 rounded-md bg-base-200/50 px-2 py-0.5 text-[11px] font-medium text-base-content/50 transition-colors hover:bg-base-200 hover:text-base-content"
                aria-label={t('conflict_manager.folder_info')}
                aria-controls={detailPanelId}
                aria-expanded={isInfoOpen}
                aria-haspopup="dialog"
                onClick={(event) => {
                  event.stopPropagation();
                  setInfoPortalTarget(event.currentTarget.closest('dialog') ?? document.body);
                  setIsInfoOpen((open) => !open);
                }}
              >
                <Info size={12} aria-hidden="true" />
                {formatBytes(detail.total_size)} ·{' '}
                {t('conflict_manager.file_count', { count: detail.file_count })}
                {detail.partial && (
                  <span className="ml-0.5 text-warning">
                    ({t('conflict_manager.partial_details')})
                  </span>
                )}
              </button>
            ) : (
              <span className="text-[11px] font-medium text-base-content/50">
                {t('conflict_manager.details_unavailable')}
              </span>
            )}
          </div>
        </div>

        {!isKeep && (
          <fieldset className="mt-4 ml-8 flex flex-col gap-3 border-l-2 border-base-content/10 pl-4 pb-1">
            <legend className="sr-only">{t('conflict_manager.action')}</legend>
            <div
              className="inline-flex w-fit rounded-lg border border-base-content/10 bg-base-300/40 p-1"
              role="group"
              aria-label={t('conflict_manager.action_for', { name: candidate.folder_name })}
            >
              <button
                type="button"
                className={`inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium transition-colors ${
                  action === 'rename'
                    ? 'bg-primary text-primary-content shadow-sm'
                    : 'text-base-content/60 hover:bg-base-content/10 hover:text-base-content'
                }`}
                aria-pressed={action === 'rename'}
                disabled={disabled}
                onClick={() => onActionChange('rename')}
              >
                <Pencil size={13} aria-hidden="true" />
                {t('conflict_manager.rename_action')}
              </button>
              <button
                type="button"
                className={`inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium transition-colors ${
                  action === 'trash'
                    ? 'bg-error text-error-content shadow-sm'
                    : 'text-base-content/60 hover:bg-base-content/10 hover:text-base-content'
                }`}
                aria-pressed={action === 'trash'}
                disabled={disabled}
                onClick={() => onActionChange('trash')}
              >
                <Trash2 size={13} aria-hidden="true" />
                {t('conflict_manager.mark_as_trash')}
              </button>
            </div>

            {action === 'rename' ? (
              <label className="flex w-full max-w-md flex-col items-stretch gap-0">
                <span className="label sr-only">{t('conflict_manager.new_base_name')}</span>
                <input
                  ref={inputRef}
                  className={`input input-sm input-bordered w-full focus-visible:outline-primary ${error ? 'input-error' : ''}`}
                  value={value}
                  onChange={(e) => onChange(e.target.value)}
                  onBlur={onBlur}
                  disabled={disabled}
                  aria-invalid={Boolean(error)}
                  aria-label={t('conflict_manager.new_base_name')}
                />
                {error && (
                  <span className="mt-1 block w-full break-words whitespace-normal text-xs leading-snug text-error">
                    {error}
                  </span>
                )}
              </label>
            ) : (
              <p className="flex max-w-md items-center gap-2 text-xs text-error/80">
                <Trash2 size={14} aria-hidden="true" />
                {t('conflict_manager.trash_pending')}
              </p>
            )}
          </fieldset>
        )}
      </article>
      {isInfoOpen &&
        detail &&
        infoPortalTarget &&
        createPortal(
          <FolderConflictDetailPanel
            candidate={candidate}
            detail={detail}
            panelId={detailPanelId}
            panelRef={infoPanelRef}
            position={infoPosition}
          />,
          infoPortalTarget,
        )}
    </>
  );
}
