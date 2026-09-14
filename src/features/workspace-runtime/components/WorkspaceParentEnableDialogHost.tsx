import { CheckCircle2, FolderOpen, Lock, PowerOff } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { WorkspaceParentEnableRequirement } from '@/entities/workspace';
import { closeWorkspaceDialog } from '../state/workspaceDialogs';
import { useWorkspaceRuntimeSelector } from '../state/workspaceStoreBridge';
import { useWorkspaceSwitchActions } from '../actions/useWorkspaceSwitchActions';

interface WorkspaceParentEnableDialogProps {
  open: boolean;
  requirement: WorkspaceParentEnableRequirement;
  isSubmitting: boolean;
  onConfirm: () => void;
  onClose: () => void;
}

function ImpactList({
  title,
  items,
  tone,
}: {
  title: string;
  items: WorkspaceParentEnableRequirement['will_activate'];
  tone: 'activate' | 'disabled';
}) {
  const Icon = tone === 'activate' ? CheckCircle2 : PowerOff;
  const iconClass = tone === 'activate' ? 'text-success' : 'text-base-content/50';

  return (
    <section className="rounded-lg border border-base-content/10">
      <h4 className="flex items-center gap-2 border-b border-base-content/10 bg-base-200/60 px-3 py-2 text-xs font-semibold text-base-content">
        <Icon size={14} className={iconClass} />
        {title}
      </h4>
      <ul className="max-h-36 divide-y divide-base-content/5 overflow-y-auto">
        {items.map((item) => (
          <li key={item.path} className="flex min-w-0 items-center gap-2 px-3 py-2">
            <FolderOpen size={13} className="shrink-0 text-base-content/45" />
            <span className="min-w-0 truncate text-sm text-base-content/85">{item.name}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}

export function WorkspaceParentEnableDialog({
  open,
  requirement,
  isSubmitting,
  onConfirm,
  onClose,
}: WorkspaceParentEnableDialogProps) {
  const { t } = useTranslation('common');
  const dialogRef = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) {
      return;
    }
    if (open && !dialog.open) {
      dialog.showModal();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  }, [open]);

  return (
    <dialog
      ref={dialogRef}
      className="modal modal-bottom sm:modal-middle"
      aria-labelledby="workspace-parent-enable-title"
      aria-describedby="workspace-parent-enable-description"
      onCancel={(event) => {
        event.preventDefault();
        if (!isSubmitting) {
          onClose();
        }
      }}
    >
      <div className="modal-box max-w-lg border border-base-content/10 bg-base-100 shadow-2xl">
        <div className="flex items-start gap-3">
          <div className="rounded-lg bg-warning/15 p-2 text-warning" aria-hidden="true">
            <Lock size={20} />
          </div>
          <div className="min-w-0">
            <h3
              id="workspace-parent-enable-title"
              className="text-base font-semibold text-base-content"
            >
              {t('parent_enable_dialog.title')}
            </h3>
            <p
              id="workspace-parent-enable-description"
              className="mt-1 text-sm text-base-content/70"
            >
              {t('parent_enable_dialog.target_blocked', {
                name: requirement.requested_target.name,
              })}
            </p>
          </div>
        </div>

        <section className="mt-4 rounded-lg border border-warning/30 bg-warning/10 p-3">
          <h4 className="text-xs font-semibold text-warning">
            {t('parent_enable_dialog.parents_required', { count: requirement.parents.length })}
          </h4>
          <ul className="mt-2 space-y-1.5">
            {requirement.parents.map((parent) => (
              <li
                key={parent.path}
                className="flex min-w-0 items-center gap-2 text-sm text-base-content"
              >
                <Lock size={13} className="shrink-0 text-warning" />
                <span className="truncate">{parent.name}</span>
              </li>
            ))}
          </ul>
        </section>

        <div className="mt-4 space-y-3">
          {requirement.will_activate.length > 0 && (
            <ImpactList
              title={t('parent_enable_dialog.will_activate', {
                count: requirement.will_activate.length,
              })}
              items={requirement.will_activate}
              tone="activate"
            />
          )}
          {requirement.stay_disabled.length > 0 && (
            <ImpactList
              title={t('parent_enable_dialog.stay_disabled', {
                count: requirement.stay_disabled.length,
              })}
              items={requirement.stay_disabled}
              tone="disabled"
            />
          )}
        </div>

        <div className="modal-action mt-5">
          <button className="btn btn-sm btn-ghost" disabled={isSubmitting} onClick={onClose}>
            {t('actions.cancel')}
          </button>
          <button className="btn btn-sm btn-warning" disabled={isSubmitting} onClick={onConfirm}>
            {isSubmitting ? (
              <span className="loading loading-spinner loading-xs" />
            ) : (
              <Lock size={14} />
            )}
            {t('parent_enable_dialog.confirm')}
          </button>
        </div>
      </div>
      <form method="dialog" className="modal-backdrop">
        <button disabled={isSubmitting} onClick={onClose}>
          {t('actions.close')}
        </button>
      </form>
    </dialog>
  );
}

export function WorkspaceParentEnableDialogHost() {
  const dialogState = useWorkspaceRuntimeSelector((state) => state.dialogState);
  const { resolveParentEnable } = useWorkspaceSwitchActions();
  const [isSubmitting, setIsSubmitting] = useState(false);

  if (dialogState.kind !== 'folderEnableParent') {
    return null;
  }

  return (
    <WorkspaceParentEnableDialog
      open
      requirement={dialogState.requirement}
      isSubmitting={isSubmitting}
      onClose={() => closeWorkspaceDialog('folderEnableParent')}
      onConfirm={() => {
        void (async () => {
          setIsSubmitting(true);
          try {
            await resolveParentEnable();
          } finally {
            setIsSubmitting(false);
          }
        })();
      }}
    />
  );
}
