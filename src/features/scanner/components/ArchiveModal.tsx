import { useCallback, useMemo, useRef, useEffect } from 'react';
import { AlertTriangle, Package } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import ArchiveActionFooter from './ArchiveActionFooter';
import ArchiveCollisionConfirm from './ArchiveCollisionConfirm';
import ArchiveExtractionOptions from './ArchiveExtractionOptions';
import ArchiveList from './ArchiveList';
import ArchiveStopConfirm from './ArchiveStopConfirm';
import type { ArchiveModalProps } from './archiveModalTypes';
import { useArchiveModalState } from './useArchiveModalState';

export default function ArchiveModal({
  archives,
  isOpen,
  onExtract,
  onSkip,
  isExtracting,
  error,
  passwordError,
  extractProgress,
  fileProgress,
  onStop,
  existingFolders,
  targetObjectName,
}: ArchiveModalProps) {
  const { t } = useTranslation(['scanner', 'common']);
  const dialogRef = useRef<HTMLDialogElement>(null);
  const resolvedExistingFolders = useMemo(() => existingFolders ?? [], [existingFolders]);
  const validationMessages = useMemo(
    () => ({
      empty: t('extract.validation.empty'),
      illegal: t('extract.validation.illegal'),
      reserved: t('extract.validation.reserved'),
    }),
    [t],
  );
  const archiveState = useArchiveModalState({
    archives,
    existingFolders: resolvedExistingFolders,
    validationMessages,
  });

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) {
      return;
    }

    if (isOpen) {
      dialog.showModal();
      return;
    }

    dialog.close();
  }, [isOpen]);

  const submitExtract = useCallback(() => {
    archiveState.setShowOverwriteConfirm(false);
    void onExtract(
      Array.from(archiveState.selectedPaths),
      archiveState.passwords,
      archiveState.buildExtractOptions(),
    );
  }, [archiveState, onExtract]);

  const handleExtractClick = useCallback(() => {
    if (archiveState.overwriteTargets.length > 0) {
      archiveState.setShowOverwriteConfirm(true);
      return;
    }

    submitExtract();
  }, [archiveState, submitExtract]);

  const handleConfirmStop = useCallback(() => {
    archiveState.setShowStopConfirm(false);
    onStop();
  }, [archiveState, onStop]);

  if (archives.length === 0) {
    return null;
  }

  return (
    <dialog ref={dialogRef} className="modal modal-bottom sm:modal-middle" onClose={onSkip}>
      <div className="modal-box w-11/12 max-w-2xl bg-base-100 p-0 overflow-hidden flex flex-col max-h-[90vh]">
        <div className="bg-base-200 p-4 flex items-center gap-3 border-b border-base-300 shrink-0">
          <div className="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center text-primary">
            <Package className="w-6 h-6" />
          </div>
          <div>
            <h3 className="font-bold text-lg">{t('scanner:extract.title')}</h3>
            {targetObjectName ? (
              <p className="text-xs text-base-content/80 mt-0.5 flex flex-col gap-1">
                <span>{t('scanner:extract.import_to', { name: targetObjectName })}</span>
                <span className="text-[10px] text-warning/80 flex items-center gap-1">
                  <AlertTriangle className="w-3 h-3" />
                  {t('scanner:extract.compatibility_check')}
                </span>
              </p>
            ) : (
              <p className="text-xs text-base-content/60">
                {t('scanner:extract.found_count', { count: archives.length })}
              </p>
            )}
          </div>
        </div>

        <div className="p-4 space-y-4 overflow-y-auto">
          {error && !passwordError && (
            <div role="alert" className="alert alert-error text-sm py-2">
              <AlertTriangle className="w-4 h-4 shrink-0" />
              <span>{error}</span>
            </div>
          )}

          <ArchiveList
            encrypted={archiveState.groups.encrypted}
            unencrypted={archiveState.groups.unencrypted}
            selectedPaths={archiveState.selectedPaths}
            passwords={archiveState.passwords}
            passwordError={passwordError}
            folderNames={archiveState.folderNames}
            editingPath={archiveState.editingPath}
            duplicateNames={archiveState.duplicateNames}
            onToggleSelection={archiveState.toggleSelection}
            onPasswordChange={archiveState.setPasswordForPath}
            onFolderNameChange={archiveState.setFolderName}
            onEditingPathChange={archiveState.setEditingPath}
            validateFolderName={archiveState.validateArchiveFolderName}
          />

          <ArchiveExtractionOptions
            autoRename={archiveState.autoRename}
            disableByDefault={archiveState.disableByDefault}
            unpackNested={archiveState.unpackNested}
            hasNestedArchives={archiveState.hasNestedArchives}
            onAutoRenameChange={archiveState.setAutoRename}
            onDisableByDefaultChange={archiveState.setDisableByDefault}
            onUnpackNestedChange={archiveState.setUnpackNested}
          />
        </div>

        <ArchiveActionFooter
          isExtracting={isExtracting}
          selectedCount={archiveState.selectedCount}
          hasValidationErrors={archiveState.hasValidationErrors}
          extractProgress={extractProgress}
          fileProgress={fileProgress}
          onSkip={onSkip}
          onExtract={handleExtractClick}
          onRequestStop={() => archiveState.setShowStopConfirm(true)}
        />

        <ArchiveStopConfirm
          isOpen={archiveState.showStopConfirm}
          onCancel={() => archiveState.setShowStopConfirm(false)}
          onConfirm={handleConfirmStop}
        />
        <ArchiveCollisionConfirm
          isOpen={archiveState.showOverwriteConfirm}
          overwriteTargets={archiveState.overwriteTargets}
          onCancel={() => archiveState.setShowOverwriteConfirm(false)}
          onConfirm={submitExtract}
        />
      </div>
      <form method="dialog" className="modal-backdrop">
        <button onClick={onSkip}>{t('common:actions.close')}</button>
      </form>
    </dialog>
  );
}
