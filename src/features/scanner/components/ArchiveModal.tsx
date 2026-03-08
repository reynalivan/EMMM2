import { useRef, useState, useEffect, useMemo, useCallback } from 'react';
import { Package, Lock, AlertTriangle, CheckCircle2, Pencil } from 'lucide-react';
import type { ArchiveInfo } from '../../../types/scanner';
import ArchiveFileTree from './ArchiveFileTree';

interface ExtractOptions {
  autoRename: boolean;
  disableByDefault: boolean;
  folderNames: Record<string, string>;
  unpackNested: boolean;
}

interface Props {
  archives: ArchiveInfo[];
  isOpen: boolean;
  onExtract: (
    selectedPaths: string[],
    passwords: Record<string, string>,
    options?: ExtractOptions,
  ) => Promise<void>;
  onSkip: () => void;
  isExtracting: boolean;
  error?: string | null;
  /** #5: Per-archive password error for inline retry */
  passwordError?: { path: string; message: string } | null;
  extractProgress?: { current: number; total: number } | null;
  /** Per-file streaming progress within the current archive */
  fileProgress?: { fileName: string; fileIndex: number; totalFiles: number } | null;
  onStop: () => void;
  /** #6: List of folder names that already exist on disk (for overwrite confirm) */
  existingFolders?: string[];
  /** Target object name if extracting into a specific object */
  targetObjectName?: string;
}

/** Derive a default folder name from an archive filename (strip extension). */
function stemName(archiveName: string): string {
  const dot = archiveName.lastIndexOf('.');
  return dot > 0 ? archiveName.slice(0, dot) : archiveName;
}

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
  existingFolders = [],
  targetObjectName,
}: Props) {
  const dialogRef = useRef<HTMLDialogElement>(null);

  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(() => {
    const validPaths = archives
      .filter((a) => a.file_count > 0 && a.has_ini !== false)
      .map((a) => a.path);
    return new Set(validPaths);
  });
  const [passwords, setPasswords] = useState<Record<string, string>>({});
  const [autoRename, setAutoRename] = useState(true);
  const [disableByDefault, setDisableByDefault] = useState(true);
  const [folderNames, setFolderNames] = useState<Record<string, string>>(() => {
    const map: Record<string, string> = {};
    for (const a of archives) {
      map[a.path] = stemName(a.name);
    }
    return map;
  });
  const [editingPath, setEditingPath] = useState<string | null>(null);
  const [showStopConfirm, setShowStopConfirm] = useState(false);
  const [showOverwriteConfirm, setShowOverwriteConfirm] = useState(false);

  // Read initial from a global setting later, hardcode true for now
  const [unpackNested, setUnpackNested] = useState(true);

  // Check if ANY archive has nested archives
  const hasNestedArchives = useMemo(() => {
    return archives.some((a) => a.contains_nested_archives);
  }, [archives]);

  // Group archives by encryption
  const { encrypted, unencrypted } = useMemo(() => {
    const enc: ArchiveInfo[] = [];
    const unenc: ArchiveInfo[] = [];
    for (const a of archives) {
      if (a.is_encrypted) enc.push(a);
      else unenc.push(a);
    }
    return { encrypted: enc, unencrypted: unenc };
  }, [archives]);

  // Detect duplicate folder names among selected archives
  const duplicateNames = useMemo(() => {
    const counts = new Map<string, number>();
    for (const path of selectedPaths) {
      const name = (folderNames[path] || '').toLowerCase();
      if (name) counts.set(name, (counts.get(name) || 0) + 1);
    }
    const dupes = new Set<string>();
    for (const [name, count] of counts) {
      if (count > 1) dupes.add(name);
    }
    return dupes;
  }, [selectedPaths, folderNames]);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;

    if (isOpen) {
      dialog.showModal();
    } else {
      dialog.close();
    }
  }, [isOpen]);

  const toggleSelection = (path: string) => {
    const next = new Set(selectedPaths);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    setSelectedPaths(next);
  };

  const setPasswordForPath = (path: string, pw: string) => {
    setPasswords((prev) => ({ ...prev, [path]: pw }));
  };

  const setFolderName = useCallback((path: string, name: string) => {
    setFolderNames((prev) => ({ ...prev, [path]: name }));
  }, []);

  // #6: Compute which selected folders would overwrite existing ones
  const overwriteTargets = useMemo(() => {
    if (autoRename || existingFolders.length === 0) return [];
    return Array.from(selectedPaths)
      .map((p) => folderNames[p] || stemName(archives.find((a) => a.path === p)?.name || ''))
      .filter((name) => existingFolders.includes(name));
  }, [autoRename, existingFolders, selectedPaths, folderNames, archives]);

  const handleExtract = () => {
    // #6: If overwrite mode and targets exist, show confirmation first
    if (overwriteTargets.length > 0) {
      setShowOverwriteConfirm(true);
      return;
    }
    doExtract();
  };

  const doExtract = () => {
    setShowOverwriteConfirm(false);
    onExtract(Array.from(selectedPaths), passwords, {
      autoRename,
      disableByDefault,
      folderNames,
      unpackNested,
    });
  };

  const isDuplicate = (path: string) => {
    const name = (folderNames[path] || '').toLowerCase();
    return duplicateNames.has(name);
  };

  // E1: Folder name validation — block illegal chars and empty names
  const validateFolderName = useCallback((name: string): string | null => {
    const ILLEGAL_CHARS = /[<>:"/\\|?*]/;
    const WINDOWS_RESERVED = /^(CON|PRN|AUX|NUL|COM[0-9]|LPT[0-9])$/i;
    if (!name.trim()) return 'Name cannot be empty';
    if (ILLEGAL_CHARS.test(name)) return 'Contains illegal characters';
    if (WINDOWS_RESERVED.test(name.trim())) return 'Reserved Windows name';
    return null;
  }, []);

  const hasValidationErrors = useMemo(() => {
    for (const path of selectedPaths) {
      const name = folderNames[path] || '';
      if (validateFolderName(name)) return true;
    }
    return false;
  }, [selectedPaths, folderNames, validateFolderName]);

  if (!archives.length) return null;

  /** Render a single archive row with folder name preview */
  const renderArchiveRow = (archive: ArchiveInfo, isEncryptedGroup: boolean) => {
    const isEmpty = archive.file_count === 0 || archive.has_ini === false;
    const titleAttr = isEmpty ? 'Archive contains no mod files' : undefined;
    const isSelected = selectedPaths.has(archive.path);
    const nameDuplicate = isDuplicate(archive.path);
    const nameError = validateFolderName(folderNames[archive.path] || '');
    const isEditing = editingPath === archive.path;

    return (
      <tr
        key={archive.path}
        className={`hover:bg-base-200/50 ${isEmpty ? 'opacity-50' : ''}`}
        title={titleAttr}
      >
        {/* Checkbox */}
        <td className="w-10 text-center">
          <label>
            <input
              type="checkbox"
              className={`checkbox checkbox-sm ${isEncryptedGroup ? 'checkbox-warning' : 'checkbox-primary'} disabled:opacity-50 cursor-pointer disabled:cursor-not-allowed`}
              checked={isSelected}
              onChange={() => toggleSelection(archive.path)}
              disabled={isEmpty}
              title={titleAttr}
            />
          </label>
        </td>

        {/* Archive name + extension */}
        <td>
          <div className="flex flex-col">
            <div className="flex items-center gap-2">
              <span className="font-medium text-sm">{archive.name}</span>
              {archive.contains_nested_archives && (
                <div
                  className="badge badge-primary badge-outline badge-xs opacity-70 cursor-help tooltip tooltip-right flex gap-1 items-center"
                  data-tip="Contains zip/rar inside. Will be unpacked if Auto-unpack is checked."
                >
                  <Package className="w-3 h-3" />
                  Nested
                </div>
              )}
            </div>
            <div className="flex items-center gap-2">
              <span className="text-[10px] text-base-content/50 uppercase">
                {archive.extension}
              </span>
              <span className="text-[10px] font-mono opacity-50">
                {(archive.size_bytes / 1024 / 1024).toFixed(2)} MB
              </span>
            </div>
            {/* Password input for encrypted */}
            {isEncryptedGroup && (
              <div className="flex flex-col gap-0.5">
                <input
                  type="password"
                  placeholder="Key"
                  className={`input input-xs input-bordered w-full max-w-50 bg-base-100 mt-1 ${passwordError?.path === archive.path ? 'input-error' : ''}`}
                  value={passwords[archive.path] || ''}
                  onChange={(e) => setPasswordForPath(archive.path, e.target.value)}
                  disabled={!isSelected || isEmpty}
                />
                {passwordError?.path === archive.path && (
                  <span className="text-[10px] text-error">{passwordError.message}</span>
                )}
              </div>
            )}
            {/* #4: File tree preview */}
            {archive.entries && archive.entries.length > 0 && (
              <ArchiveFileTree entries={archive.entries} totalCount={archive.file_count} />
            )}
          </div>
        </td>

        {/* Folder name preview */}
        <td className="text-right">
          {isEditing ? (
            <input
              type="text"
              className={`input input-xs input-bordered w-full max-w-40 text-right ${nameError ? 'input-error' : nameDuplicate ? 'input-warning' : ''}`}
              value={folderNames[archive.path] || ''}
              onChange={(e) => setFolderName(archive.path, e.target.value)}
              onBlur={() => setEditingPath(null)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') setEditingPath(null);
              }}
              title={nameError || undefined}
              autoFocus
            />
          ) : (
            <div
              className={`flex items-center justify-end gap-1 cursor-pointer group ${nameError ? 'text-error' : nameDuplicate ? 'text-warning' : 'text-base-content/60'}`}
              onClick={() => !isEmpty && setEditingPath(archive.path)}
              title={
                nameError ||
                (nameDuplicate
                  ? 'Duplicate name — will auto-rename or overwrite'
                  : 'Click to rename')
              }
            >
              <span className="text-xs font-mono truncate max-w-36">
                {folderNames[archive.path] || stemName(archive.name)}
              </span>
              <Pencil className="w-3 h-3 opacity-0 group-hover:opacity-60 shrink-0" />
            </div>
          )}
        </td>

        {/* Status icon */}
        <td className="w-10 text-center">
          {isEmpty ? (
            <div
              className="tooltip tooltip-left text-warning"
              data-tip="Archive contains no mod files"
            >
              <AlertTriangle className="w-4 h-4 cursor-help" />
            </div>
          ) : nameDuplicate ? (
            <div className="tooltip tooltip-left text-warning" data-tip="Duplicate folder name">
              <AlertTriangle className="w-4 h-4" />
            </div>
          ) : isEncryptedGroup ? (
            <div className="tooltip tooltip-left" data-tip="Requires password to extract">
              <Lock className="w-4 h-4 text-warning/70" />
            </div>
          ) : null}
        </td>
      </tr>
    );
  };

  return (
    <dialog ref={dialogRef} className="modal modal-bottom sm:modal-middle" onClose={onSkip}>
      <div className="modal-box w-11/12 max-w-2xl bg-base-100 p-0 overflow-hidden flex flex-col max-h-[90vh]">
        {/* Header */}
        <div className="bg-base-200 p-4 flex items-center gap-3 border-b border-base-300 shrink-0">
          <div className="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center text-primary">
            <Package className="w-6 h-6" />
          </div>
          <div>
            <h3 className="font-bold text-lg">Archives Detected</h3>
            {targetObjectName ? (
              <p className="text-xs text-base-content/80 mt-0.5 flex flex-col gap-1">
                <span>
                  Importing to: <span className="font-bold text-primary">{targetObjectName}</span>
                </span>
                <span className="text-[10px] text-warning/80 flex items-center gap-1">
                  <AlertTriangle className="w-3 h-3" />
                  Archives will be checked for compatibility after extraction.
                </span>
              </p>
            ) : (
              <p className="text-xs text-base-content/60">
                Found {archives.length} archive(s) ready for import.
              </p>
            )}
          </div>
        </div>

        {/* Content (Scrollable) */}
        <div className="p-4 space-y-4 overflow-y-auto">
          {error && !passwordError && (
            <div role="alert" className="alert alert-error text-sm py-2">
              <AlertTriangle className="w-4 h-4 shrink-0" />
              <span>{error}</span>
            </div>
          )}

          {/* Unencrypted Group */}
          {unencrypted.length > 0 && (
            <div className="space-y-2">
              <div className="flex items-center gap-2 text-sm font-semibold text-success px-1">
                <CheckCircle2 className="w-4 h-4" />
                No Password Required ({unencrypted.length})
              </div>
              <div className="overflow-hidden border border-base-300 rounded-lg">
                <table className="table table-sm">
                  <tbody className="divide-y divide-base-300">
                    {unencrypted.map((a) => renderArchiveRow(a, false))}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {/* Encrypted Group */}
          {encrypted.length > 0 && (
            <div className="space-y-2">
              <div className="flex items-center gap-2 text-sm font-semibold text-warning px-1">
                <Lock className="w-4 h-4" />
                Password Protected ({encrypted.length})
              </div>
              <div className="overflow-hidden border border-warning/30 rounded-lg">
                <table className="table table-sm">
                  <tbody className="divide-y divide-warning/10">
                    {encrypted.map((a) => renderArchiveRow(a, true))}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {/* Options */}
          <div className="space-y-2">
            {/* Auto-Rename / Overwrite Toggle */}
            <div
              className={`form-control rounded-lg p-3 border ${autoRename ? 'bg-base-200/50 border-base-300' : 'bg-error/5 border-error/30'}`}
            >
              <label className="label cursor-pointer justify-start gap-3 py-0">
                <input
                  type="checkbox"
                  className="toggle toggle-sm toggle-primary"
                  checked={autoRename}
                  onChange={(e) => setAutoRename(e.target.checked)}
                />
                <div className="flex flex-col">
                  {autoRename ? (
                    <span className="label-text text-sm">Auto rename folder if name conflict</span>
                  ) : (
                    <>
                      <span className="label-text text-sm text-error">
                        Overwrite existing folder
                      </span>
                      <span className="text-[10px] text-error/70">
                        Original files will be permanently overwritten
                        <span className="badge badge-error badge-xs ml-2 align-middle">
                          Not Recommended
                        </span>
                      </span>
                    </>
                  )}
                </div>
              </label>
            </div>

            {/* Disabled by default */}
            <div className="form-control bg-base-200/50 rounded-lg p-3 border border-base-300">
              <label className="label cursor-pointer justify-start gap-3 py-0">
                <input
                  type="checkbox"
                  className="checkbox checkbox-sm checkbox-primary"
                  checked={disableByDefault}
                  onChange={(e) => setDisableByDefault(e.target.checked)}
                />
                <span className="label-text text-sm">Set as disabled after extraction</span>
              </label>
            </div>

            {hasNestedArchives && (
              <div className="form-control bg-primary/5 rounded-lg p-3 border border-primary/20">
                <label
                  className="label cursor-pointer justify-start gap-3 py-0 tooltip tooltip-right"
                  data-tip="Recursively unpack zip/rar files hidden inside the main archive"
                >
                  <input
                    type="checkbox"
                    className="checkbox checkbox-sm checkbox-primary"
                    checked={unpackNested}
                    onChange={(e) => setUnpackNested(e.target.checked)}
                  />
                  <div className="flex flex-col text-left">
                    <span className="label-text text-sm font-medium">
                      Auto-unpack nested archives (Modpacks)
                    </span>
                  </div>
                </label>
              </div>
            )}
          </div>
        </div>

        {/* Actions */}
        <div className="modal-action bg-base-200 p-4 m-0 flex flex-col gap-2 border-t border-base-300 shrink-0">
          {isExtracting && extractProgress && extractProgress.total > 0 && (
            <div className="flex flex-col gap-1 w-full">
              <div className="flex justify-between text-xs text-base-content/60">
                <span>
                  Extracting archive {extractProgress.current} of {extractProgress.total}...
                </span>
                <span>{Math.round((extractProgress.current / extractProgress.total) * 100)}%</span>
              </div>
              <progress
                className="progress progress-primary w-full"
                value={extractProgress.current}
                max={extractProgress.total}
              />
              {fileProgress && (
                <div className="flex flex-col gap-0.5 mt-1">
                  <div className="flex justify-between text-[10px] text-base-content/40">
                    <span className="truncate max-w-70" title={fileProgress.fileName}>
                      {fileProgress.fileName || 'Extracting...'}
                    </span>
                    {fileProgress.totalFiles > 0 && (
                      <span>
                        {fileProgress.fileIndex}/{fileProgress.totalFiles} files
                      </span>
                    )}
                  </div>
                  {fileProgress.totalFiles > 0 && (
                    <progress
                      className="progress progress-accent progress-xs w-full opacity-60"
                      value={fileProgress.fileIndex}
                      max={fileProgress.totalFiles}
                    />
                  )}
                </div>
              )}
            </div>
          )}
          <div className="flex justify-between items-center w-full">
            <button className="btn btn-ghost btn-sm" onClick={onSkip} disabled={isExtracting}>
              Skip Extraction
            </button>
            <div className="flex gap-2 items-center">
              <div className="text-xs text-base-content/50 mr-2">{selectedPaths.size} selected</div>

              {isExtracting ? (
                <button className="btn btn-error btn-sm" onClick={() => setShowStopConfirm(true)}>
                  Stop
                </button>
              ) : (
                <button
                  className="btn btn-primary btn-sm"
                  onClick={handleExtract}
                  disabled={selectedPaths.size === 0 || hasValidationErrors}
                >
                  Extract Selected
                </button>
              )}
            </div>
          </div>
        </div>

        {/* Confirmation Overlay for Stop */}
        {showStopConfirm && (
          <div className="absolute inset-0 bg-base-100/90 backdrop-blur-sm flex items-center justify-center z-50 rounded-lg">
            <div className="bg-base-200 border border-base-300 p-6 rounded-xl shadow-xl max-w-sm flex flex-col gap-4">
              <div className="flex items-center gap-3 text-error">
                <AlertTriangle size={24} />
                <h3 className="font-bold text-lg">Stop Extraction?</h3>
              </div>
              <p className="text-sm">
                Are you sure you want to stop? The currently extracting archive will be reverted,
                but any already-extracted archives will remain.
              </p>
              <div className="flex justify-end gap-2 mt-2">
                <button className="btn btn-ghost btn-sm" onClick={() => setShowStopConfirm(false)}>
                  Cancel
                </button>
                <button
                  className="btn btn-error btn-sm"
                  onClick={() => {
                    setShowStopConfirm(false);
                    onStop();
                  }}
                >
                  Yes, Stop
                </button>
              </div>
            </div>
          </div>
        )}

        {/* #6: Confirmation Overlay for Overwrite */}
        {showOverwriteConfirm && (
          <div className="absolute inset-0 bg-base-100/90 backdrop-blur-sm flex items-center justify-center z-50 rounded-lg">
            <div className="bg-base-200 border border-base-300 p-6 rounded-xl shadow-xl max-w-sm flex flex-col gap-4">
              <div className="flex items-center gap-3 text-warning">
                <AlertTriangle size={24} />
                <h3 className="font-bold text-lg">Overwrite Existing Folders?</h3>
              </div>
              <p className="text-sm">
                The following {overwriteTargets.length} folder(s) already exist and will be
                permanently replaced:
              </p>
              <ul className="text-sm list-disc list-inside max-h-32 overflow-y-auto bg-base-300/50 rounded-lg p-2">
                {overwriteTargets.map((name) => (
                  <li key={name} className="font-mono text-warning">
                    {name}
                  </li>
                ))}
              </ul>
              <div className="flex justify-end gap-2 mt-2">
                <button
                  className="btn btn-ghost btn-sm"
                  onClick={() => setShowOverwriteConfirm(false)}
                >
                  Cancel
                </button>
                <button className="btn btn-warning btn-sm" onClick={doExtract}>
                  Yes, Overwrite
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
      <form method="dialog" className="modal-backdrop">
        <button onClick={onSkip}>close</button>
      </form>
    </dialog>
  );
}
