import { useTranslation } from 'react-i18next';
import ConfirmDialog from '../../../components/ui/ConfirmDialog';
import UnsavedIniChangesModal, {
  type IniChange,
  type MetadataChange,
} from './UnsavedIniChangesModal';

interface PreviewConfirmDialogsProps {
  confirmRemoveOpen: boolean;
  confirmClearOpen: boolean;
  showUnsavedModal: boolean;
  isSaving: boolean;
  modName?: string;
  categoryName?: string;
  changedIniFields: IniChange[];
  changedMetadataFields: MetadataChange[];
  onCancelRemove: () => void;
  onConfirmRemove: () => void;
  onCancelClear: () => void;
  onConfirmClear: () => void;
  onCancelUnsaved: () => void;
  onDiscardUnsaved: () => void;
  onSaveUnsaved: () => Promise<void>;
}

export default function PreviewConfirmDialogs({
  confirmRemoveOpen,
  confirmClearOpen,
  showUnsavedModal,
  isSaving,
  modName,
  categoryName,
  changedIniFields,
  changedMetadataFields,
  onCancelRemove,
  onConfirmRemove,
  onCancelClear,
  onConfirmClear,
  onCancelUnsaved,
  onDiscardUnsaved,
  onSaveUnsaved,
}: PreviewConfirmDialogsProps) {
  const { t } = useTranslation(['preview', 'common']);

  return (
    <>
      <ConfirmDialog
        open={confirmRemoveOpen}
        title={t('preview:gallery.menu.remove_current')}
        message={t('preview:gallery.remove_confirm_message')}
        confirmLabel={t('common:actions.remove')}
        cancelLabel={t('common:actions.cancel')}
        danger
        onCancel={onCancelRemove}
        onConfirm={onConfirmRemove}
      />

      <ConfirmDialog
        open={confirmClearOpen}
        title={t('preview:gallery.menu.clear_all')}
        message={t('preview:gallery.clear_all_confirm_message')}
        confirmLabel={t('preview:gallery.menu.clear_all')}
        cancelLabel={t('common:actions.cancel')}
        danger
        onCancel={onCancelClear}
        onConfirm={onConfirmClear}
      />

      <UnsavedIniChangesModal
        open={showUnsavedModal}
        isSaving={isSaving}
        modName={modName}
        categoryName={categoryName}
        changedIniFields={changedIniFields}
        changedMetadataFields={changedMetadataFields}
        onCancel={onCancelUnsaved}
        onDiscard={onDiscardUnsaved}
        onSave={() => {
          void onSaveUnsaved();
        }}
      />
    </>
  );
}
