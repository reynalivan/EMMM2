import { CheckCircle2, Lock } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { ArchiveInfo } from '../../../types/scanner';
import ArchiveRow from './ArchiveRow';

interface ArchiveListProps {
  encrypted: ArchiveInfo[];
  unencrypted: ArchiveInfo[];
  selectedPaths: Set<string>;
  passwords: Record<string, string>;
  passwordError?: { path: string; message: string } | null;
  folderNames: Record<string, string>;
  editingPath: string | null;
  duplicateNames: Set<string>;
  onToggleSelection: (path: string) => void;
  onPasswordChange: (path: string, password: string) => void;
  onFolderNameChange: (path: string, name: string) => void;
  onEditingPathChange: (path: string | null) => void;
  validateFolderName: (name: string) => string | null;
}

function ArchiveGroup({
  archives,
  isEncryptedGroup,
  selectedPaths,
  passwords,
  passwordError,
  folderNames,
  editingPath,
  duplicateNames,
  onToggleSelection,
  onPasswordChange,
  onFolderNameChange,
  onEditingPathChange,
  validateFolderName,
}: ArchiveListProps & { archives: ArchiveInfo[]; isEncryptedGroup: boolean }) {
  if (archives.length === 0) {
    return null;
  }

  return (
    <div className="overflow-hidden border border-base-300 rounded-lg">
      <table className="table table-sm">
        <tbody className="divide-y divide-base-300">
          {archives.map((archive) => {
            const folderName = folderNames[archive.path] ?? '';
            return (
              <ArchiveRow
                key={archive.path}
                archive={archive}
                isEncryptedGroup={isEncryptedGroup}
                isSelected={selectedPaths.has(archive.path)}
                password={passwords[archive.path] ?? ''}
                passwordError={passwordError}
                folderName={folderName}
                isEditing={editingPath === archive.path}
                isDuplicate={duplicateNames.has(folderName.toLowerCase())}
                nameError={validateFolderName(folderName)}
                onToggleSelection={onToggleSelection}
                onPasswordChange={onPasswordChange}
                onFolderNameChange={onFolderNameChange}
                onEditingPathChange={onEditingPathChange}
              />
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

export default function ArchiveList(props: ArchiveListProps) {
  const { t } = useTranslation(['scanner']);

  return (
    <>
      {props.unencrypted.length > 0 && (
        <div className="space-y-2">
          <div className="flex items-center gap-2 text-sm font-semibold text-success px-1">
            <CheckCircle2 className="w-4 h-4" />
            {t('extract.no_password', { count: props.unencrypted.length })}
          </div>
          <ArchiveGroup {...props} archives={props.unencrypted} isEncryptedGroup={false} />
        </div>
      )}

      {props.encrypted.length > 0 && (
        <div className="space-y-2">
          <div className="flex items-center gap-2 text-sm font-semibold text-warning px-1">
            <Lock className="w-4 h-4" />
            {t('extract.password_protected', { count: props.encrypted.length })}
          </div>
          <div className="overflow-hidden border border-warning/30 rounded-lg">
            <table className="table table-sm">
              <tbody className="divide-y divide-warning/10">
                {props.encrypted.map((archive) => {
                  const folderName = props.folderNames[archive.path] ?? '';
                  return (
                    <ArchiveRow
                      key={archive.path}
                      archive={archive}
                      isEncryptedGroup
                      isSelected={props.selectedPaths.has(archive.path)}
                      password={props.passwords[archive.path] ?? ''}
                      passwordError={props.passwordError}
                      folderName={folderName}
                      isEditing={props.editingPath === archive.path}
                      isDuplicate={props.duplicateNames.has(folderName.toLowerCase())}
                      nameError={props.validateFolderName(folderName)}
                      onToggleSelection={props.onToggleSelection}
                      onPasswordChange={props.onPasswordChange}
                      onFolderNameChange={props.onFolderNameChange}
                      onEditingPathChange={props.onEditingPathChange}
                    />
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </>
  );
}
