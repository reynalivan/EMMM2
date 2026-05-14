import { AlertTriangle, Lock, Package, Pencil } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { ArchiveInfo } from '../../../types/scanner';
import { formatBytes } from '../../../utils/formatters';
import ArchiveFileTree from './ArchiveFileTree';
import { isArchiveEmpty, stemName } from './archiveModalUtils';

interface ArchiveRowProps {
  archive: ArchiveInfo;
  isEncryptedGroup: boolean;
  isSelected: boolean;
  password: string;
  passwordError?: { path: string; message: string } | null;
  folderName: string;
  isEditing: boolean;
  isDuplicate: boolean;
  nameError: string | null;
  onToggleSelection: (path: string) => void;
  onPasswordChange: (path: string, password: string) => void;
  onFolderNameChange: (path: string, name: string) => void;
  onEditingPathChange: (path: string | null) => void;
}

export default function ArchiveRow({
  archive,
  isEncryptedGroup,
  isSelected,
  password,
  passwordError,
  folderName,
  isEditing,
  isDuplicate,
  nameError,
  onToggleSelection,
  onPasswordChange,
  onFolderNameChange,
  onEditingPathChange,
}: ArchiveRowProps) {
  const { t } = useTranslation(['scanner']);
  const isEmpty = isArchiveEmpty(archive);
  const titleAttr = isEmpty ? t('extract.no_mod_files') : undefined;

  return (
    <tr
      key={archive.path}
      className={`hover:bg-base-200/50 ${isEmpty ? 'opacity-50' : ''}`}
      title={titleAttr}
    >
      <td className="w-10 text-center">
        <label>
          <input
            type="checkbox"
            className={`checkbox checkbox-sm ${isEncryptedGroup ? 'checkbox-warning' : 'checkbox-primary'} disabled:opacity-50 cursor-pointer disabled:cursor-not-allowed`}
            checked={isSelected}
            onChange={() => onToggleSelection(archive.path)}
            disabled={isEmpty}
            title={titleAttr}
          />
        </label>
      </td>

      <td>
        <div className="flex flex-col">
          <div className="flex items-center gap-2">
            <span className="font-medium text-sm">{archive.name}</span>
            {archive.contains_nested_archives && (
              <div
                className="badge badge-primary badge-outline badge-xs opacity-70 cursor-help tooltip tooltip-right flex gap-1 items-center"
                data-tip={t('extract.nested_tooltip')}
              >
                <Package className="w-3 h-3" />
                {t('extract.nested_label')}
              </div>
            )}
          </div>
          <div className="flex items-center gap-2">
            <span className="text-[10px] text-base-content/50 uppercase">{archive.extension}</span>
            <span className="text-[10px] font-mono opacity-50">
              {formatBytes(archive.size_bytes)}
            </span>
          </div>
          {isEncryptedGroup && (
            <div className="flex flex-col gap-0.5">
              <input
                type="password"
                placeholder={t('extract.password_placeholder')}
                className={`input input-xs input-bordered w-full max-w-50 bg-base-100 mt-1 ${passwordError?.path === archive.path ? 'input-error' : ''}`}
                value={password}
                onChange={(event) => onPasswordChange(archive.path, event.target.value)}
                disabled={!isSelected || isEmpty}
              />
              {passwordError?.path === archive.path && (
                <span className="text-[10px] text-error">{passwordError.message}</span>
              )}
            </div>
          )}
          {archive.entries && archive.entries.length > 0 && (
            <ArchiveFileTree entries={archive.entries} totalCount={archive.file_count} />
          )}
        </div>
      </td>

      <td className="text-right">
        {isEditing ? (
          <input
            type="text"
            className={`input input-xs input-bordered w-full max-w-40 text-right ${nameError ? 'input-error' : isDuplicate ? 'input-warning' : ''}`}
            value={folderName}
            onChange={(event) => onFolderNameChange(archive.path, event.target.value)}
            onBlur={() => onEditingPathChange(null)}
            onKeyDown={(event) => {
              if (event.key === 'Enter') {
                onEditingPathChange(null);
              }
            }}
            title={nameError ?? undefined}
            autoFocus
          />
        ) : (
          <div
            className={`flex items-center justify-end gap-1 cursor-pointer group ${nameError ? 'text-error' : isDuplicate ? 'text-warning' : 'text-base-content/60'}`}
            onClick={() => {
              if (!isEmpty) {
                onEditingPathChange(archive.path);
              }
            }}
            title={
              nameError ??
              (isDuplicate ? t('extract.duplicate_name_action') : t('extract.rename_tooltip'))
            }
          >
            <span className="text-xs font-mono truncate max-w-36">
              {folderName || stemName(archive.name)}
            </span>
            <Pencil className="w-3 h-3 opacity-0 group-hover:opacity-60 shrink-0" />
          </div>
        )}
      </td>

      <td className="w-10 text-center">
        {isEmpty ? (
          <div className="tooltip tooltip-left text-warning" data-tip={t('extract.no_mod_files')}>
            <AlertTriangle className="w-4 h-4 cursor-help" />
          </div>
        ) : isDuplicate ? (
          <div className="tooltip tooltip-left text-warning" data-tip={t('extract.duplicate_name')}>
            <AlertTriangle className="w-4 h-4" />
          </div>
        ) : isEncryptedGroup ? (
          <div className="tooltip tooltip-left" data-tip={t('extract.password_required')}>
            <Lock className="w-4 h-4 text-warning/70" />
          </div>
        ) : null}
      </td>
    </tr>
  );
}
